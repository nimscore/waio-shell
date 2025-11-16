use crate::slint_callbacks::SlintCallbackContract;
use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::{
    EventSource, Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer,
    channel,
};
use layer_shika_adapters::platform::slint::ComponentHandle;
use layer_shika_adapters::platform::slint_interpreter::{
    CompilationResult, ComponentDefinition, ComponentInstance,
};
use layer_shika_adapters::{
    AppState, PopupManager, WaylandWindowConfig, WindowState, WindowingSystemFacade,
};
use layer_shika_domain::config::WindowConfig;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{PopupHandle, PopupRequest, PopupSize};
use std::cell::Cell;
use std::cell::RefCell;
use std::os::unix::io::AsFd;
use std::rc::{Rc, Weak};
use std::result::Result as StdResult;
use std::time::{Duration, Instant};

pub enum PopupCommand {
    Show(PopupRequest),
    Close(PopupHandle),
    Resize {
        handle: PopupHandle,
        width: f32,
        height: f32,
    },
}

pub struct EventLoopHandle {
    system: Weak<RefCell<WindowingSystemFacade>>,
}

impl EventLoopHandle {
    pub fn insert_source<S, F, R>(
        &self,
        source: S,
        mut callback: F,
    ) -> StdResult<RegistrationToken, Error>
    where
        S: EventSource<Ret = R> + 'static,
        F: FnMut(S::Event, &mut S::Metadata, RuntimeState<'_>) -> R + 'static,
    {
        let system = self.system.upgrade().ok_or(Error::SystemDropped)?;
        let loop_handle = system.borrow().inner_ref().event_loop_handle();

        loop_handle
            .insert_source(source, move |event, metadata, app_state| {
                let runtime_state = RuntimeState { app_state };
                callback(event, metadata, runtime_state)
            })
            .map_err(|e| {
                Error::Adapter(
                    EventLoopError::InsertSource {
                        message: format!("{e:?}"),
                    }
                    .into(),
                )
            })
    }

    pub fn add_timer<F>(&self, duration: Duration, mut callback: F) -> Result<RegistrationToken>
    where
        F: FnMut(Instant, RuntimeState<'_>) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_duration(duration);
        self.insert_source(timer, move |deadline, (), runtime_state| {
            callback(deadline, runtime_state)
        })
    }

    pub fn add_timer_at<F>(&self, deadline: Instant, mut callback: F) -> Result<RegistrationToken>
    where
        F: FnMut(Instant, RuntimeState<'_>) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_deadline(deadline);
        self.insert_source(timer, move |deadline, (), runtime_state| {
            callback(deadline, runtime_state)
        })
    }

    pub fn add_channel<T, F>(
        &self,
        mut callback: F,
    ) -> Result<(RegistrationToken, channel::Sender<T>)>
    where
        T: 'static,
        F: FnMut(T, RuntimeState<'_>) + 'static,
    {
        let (sender, receiver) = channel::channel();
        let token = self.insert_source(receiver, move |event, (), runtime_state| {
            if let channel::Event::Msg(msg) = event {
                callback(msg, runtime_state);
            }
        })?;
        Ok((token, sender))
    }

    pub fn add_fd<F, T>(
        &self,
        fd: T,
        interest: Interest,
        mode: Mode,
        mut callback: F,
    ) -> Result<RegistrationToken>
    where
        T: AsFd + 'static,
        F: FnMut(RuntimeState<'_>) + 'static,
    {
        let generic = Generic::new(fd, interest, mode);
        self.insert_source(generic, move |_readiness, _fd, runtime_state| {
            callback(runtime_state);
            Ok(PostAction::Continue)
        })
    }
}

pub struct RuntimeState<'a> {
    app_state: &'a mut AppState,
}

impl RuntimeState<'_> {
    #[must_use]
    pub fn component_instance(&self) -> Option<&ComponentInstance> {
        self.app_state
            .primary_output()
            .map(WindowState::component_instance)
    }

    pub fn all_component_instances(&self) -> impl Iterator<Item = &ComponentInstance> {
        self.app_state
            .all_outputs()
            .map(WindowState::component_instance)
    }

    fn active_or_primary_output(&self) -> Option<&WindowState> {
        self.app_state
            .active_output()
            .and_then(|key| self.app_state.get_output_by_key(key))
            .or_else(|| self.app_state.primary_output())
    }

    pub fn render_frame_if_dirty(&mut self) -> Result<()> {
        for window in self.app_state.all_outputs() {
            window.render_frame_if_dirty()?;
        }
        Ok(())
    }

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.app_state
            .primary_output()
            .and_then(WindowState::compilation_result)
    }

    pub fn show_popup(
        &mut self,
        req: PopupRequest,
        resize_sender: Option<channel::Sender<PopupCommand>>,
    ) -> Result<PopupHandle> {
        let compilation_result = self.compilation_result().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No compilation result available for popup creation".to_string(),
            })
        })?;

        let definition = compilation_result
            .component(&req.component)
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "{} component not found in compilation result",
                        req.component
                    ),
                })
            })?;

        self.close_current_popup()?;

        let is_using_active = self.app_state.active_output().is_some();
        let active_window = self.active_or_primary_output().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No active or primary output available".to_string(),
            })
        })?;

        log::info!(
            "Creating popup on {} output",
            if is_using_active { "active" } else { "primary" }
        );

        let popup_manager = active_window.popup_manager().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No popup manager available".to_string(),
            })
        })?;

        let initial_dimensions = match req.size {
            PopupSize::Fixed { w, h } => {
                log::debug!("Using fixed popup size: {}x{}", w, h);
                (w, h)
            }
            PopupSize::Content => {
                log::debug!("Using content-based sizing - will measure after instance creation");
                (2.0, 2.0)
            }
        };

        log::debug!(
            "Creating popup for '{}' with dimensions {}x{} at position ({}, {}), mode: {:?}",
            req.component,
            initial_dimensions.0,
            initial_dimensions.1,
            req.at.position().0,
            req.at.position().1,
            req.mode
        );

        let popup_handle =
            popup_manager.request_popup(req, initial_dimensions.0, initial_dimensions.1);

        let (instance, popup_key_cell) =
            Self::create_popup_instance(&definition, &popup_manager, resize_sender)?;

        popup_key_cell.set(popup_handle.key());

        if let Some(popup_window) = popup_manager.get_popup_window(popup_handle.key()) {
            popup_window.set_component_instance(instance);
        } else {
            return Err(Error::Domain(DomainError::Configuration {
                message: "Popup window not found after creation".to_string(),
            }));
        }

        Ok(popup_handle)
    }

    pub fn close_popup(&mut self, handle: PopupHandle) -> Result<()> {
        if let Some(active_window) = self.active_or_primary_output() {
            if let Some(popup_manager) = active_window.popup_manager() {
                popup_manager.close(handle)?;
            }
        }
        Ok(())
    }

    pub fn close_current_popup(&mut self) -> Result<()> {
        if let Some(active_window) = self.active_or_primary_output() {
            if let Some(popup_manager) = active_window.popup_manager() {
                popup_manager.close_current_popup();
            }
        }
        Ok(())
    }

    pub fn resize_popup(
        &mut self,
        handle: PopupHandle,
        width: f32,
        height: f32,
        resize_sender: Option<channel::Sender<PopupCommand>>,
    ) -> Result<()> {
        let active_window = self.active_or_primary_output().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No active or primary output available".to_string(),
            })
        })?;

        let popup_manager = active_window.popup_manager().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No popup manager available".to_string(),
            })
        })?;

        let Some((request, _serial)) = popup_manager.get_popup_info(handle.key()) else {
            log::debug!(
                "Ignoring resize request for non-existent popup with handle {:?}",
                handle
            );
            return Ok(());
        };

        let current_size = request.size.dimensions();
        let size_changed =
            current_size.is_none_or(|(w, h)| (w - width).abs() > 0.01 || (h - height).abs() > 0.01);

        let needs_repositioning = request.mode.center_x() || request.mode.center_y();

        if needs_repositioning && size_changed {
            log::info!(
                "Popup needs repositioning due to mode {:?} and size change - recreating with new size {}x{}",
                request.mode,
                width,
                height
            );

            self.close_popup(handle)?;

            let new_request = PopupRequest::builder(request.component)
                .at(request.at)
                .size(PopupSize::fixed(width, height))
                .mode(request.mode)
                .build();

            self.show_popup(new_request, resize_sender)?;
        } else if size_changed {
            if let Some(popup_window) = popup_manager.get_popup_window(handle.key()) {
                popup_window.request_resize(width, height);
            }
        }

        Ok(())
    }

    fn create_popup_instance(
        definition: &ComponentDefinition,
        popup_manager: &Rc<PopupManager>,
        resize_sender: Option<channel::Sender<PopupCommand>>,
    ) -> Result<(ComponentInstance, Rc<Cell<usize>>)> {
        let instance = definition.create().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to create popup instance: {}", e),
            })
        })?;

        let popup_key_cell = Rc::new(Cell::new(0));

        SlintCallbackContract::register_on_popup_component(
            &instance,
            popup_manager,
            resize_sender,
            &popup_key_cell,
        )?;

        instance.show().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to show popup instance: {}", e),
            })
        })?;

        Ok((instance, popup_key_cell))
    }
}

pub struct WindowingSystem {
    inner: Rc<RefCell<WindowingSystemFacade>>,
    popup_command_sender: channel::Sender<PopupCommand>,
    callback_contract: SlintCallbackContract,
}

impl WindowingSystem {
    pub(crate) fn new(
        component_definition: ComponentDefinition,
        compilation_result: Option<Rc<CompilationResult>>,
        config: WindowConfig,
    ) -> Result<Self> {
        let wayland_config = WaylandWindowConfig::from_domain_config(
            component_definition,
            compilation_result,
            config,
        );
        let inner = layer_shika_adapters::WaylandWindowingSystem::new(&wayland_config)?;
        let facade = WindowingSystemFacade::new(inner);
        let inner_rc = Rc::new(RefCell::new(facade));

        let (sender, receiver) = channel::channel();

        let popup_positioning_mode = Rc::new(RefCell::new(PopupPositioningMode::Center));
        let callback_contract =
            SlintCallbackContract::new(Rc::clone(&popup_positioning_mode), sender.clone());

        let system = Self {
            inner: Rc::clone(&inner_rc),
            popup_command_sender: sender,
            callback_contract,
        };

        system.setup_popup_command_handler(receiver)?;
        system.register_popup_callbacks();

        Ok(system)
    }

    fn setup_popup_command_handler(&self, receiver: channel::Channel<PopupCommand>) -> Result<()> {
        let loop_handle = self.inner.borrow().inner_ref().event_loop_handle();
        let sender_for_handler = self.popup_command_sender.clone();

        loop_handle
            .insert_source(receiver, move |event, (), app_state| {
                if let channel::Event::Msg(command) = event {
                    let mut runtime_state = RuntimeState { app_state };

                    match command {
                        PopupCommand::Show(request) => {
                            if let Err(e) =
                                runtime_state.show_popup(request, Some(sender_for_handler.clone()))
                            {
                                log::error!("Failed to show popup: {}", e);
                            }
                        }
                        PopupCommand::Close(handle) => {
                            if let Err(e) = runtime_state.close_popup(handle) {
                                log::error!("Failed to close popup: {}", e);
                            }
                        }
                        PopupCommand::Resize {
                            handle,
                            width,
                            height,
                        } => {
                            if let Err(e) = runtime_state.resize_popup(
                                handle,
                                width,
                                height,
                                Some(sender_for_handler.clone()),
                            ) {
                                log::error!("Failed to resize popup: {}", e);
                            }
                        }
                    }
                }
            })
            .map_err(|e| {
                Error::Adapter(
                    EventLoopError::InsertSource {
                        message: format!("Failed to setup popup command handler: {e:?}"),
                    }
                    .into(),
                )
            })?;

        Ok(())
    }

    fn register_popup_callbacks(&self) {
        self.with_all_component_instances(|component_instance| {
            if let Err(e) = self
                .callback_contract
                .register_on_main_component(component_instance)
            {
                log::error!("Failed to register popup callbacks on output: {}", e);
            }
        });
    }

    #[must_use]
    pub fn event_loop_handle(&self) -> EventLoopHandle {
        EventLoopHandle {
            system: Rc::downgrade(&self.inner),
        }
    }

    pub fn request_show_popup(&self, request: PopupRequest) -> Result<()> {
        self.popup_command_sender
            .send(PopupCommand::Show(request))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup show command: channel closed".to_string(),
                })
            })
    }

    pub fn request_close_popup(&self, handle: PopupHandle) -> Result<()> {
        self.popup_command_sender
            .send(PopupCommand::Close(handle))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup close command: channel closed".to_string(),
                })
            })
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.borrow_mut().run()?;
        Ok(())
    }

    pub fn with_component_instance<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&ComponentInstance) -> R,
    {
        let facade = self.inner.borrow();
        let instance = facade.component_instance()?;
        Ok(f(instance))
    }

    pub fn with_all_component_instances<F>(&self, mut f: F)
    where
        F: FnMut(&ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        for window in system.app_state().all_outputs() {
            f(window.component_instance());
        }
    }
}
