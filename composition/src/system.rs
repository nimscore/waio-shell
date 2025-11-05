use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::{
    EventSource, Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer,
    channel,
};
use layer_shika_adapters::platform::slint::{ComponentHandle, SharedString};
use layer_shika_adapters::platform::slint_interpreter::{
    CompilationResult, ComponentDefinition, ComponentInstance, Value,
};
use layer_shika_adapters::wayland::{
    config::WaylandWindowConfig,
    shell_adapter::WaylandWindowingSystem,
    surfaces::{popup_manager::PopupManager, surface_state::WindowState},
};
use layer_shika_domain::config::WindowConfig;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{
    PopupAt, PopupHandle, PopupRequest, PopupSize,
};
use std::cell::{Ref, RefCell};
use std::os::unix::io::AsFd;
use std::rc::{Rc, Weak};
use std::result::Result as StdResult;
use std::time::{Duration, Instant};

pub enum PopupCommand {
    Show(PopupRequest),
    Close(PopupHandle),
    Resize { key: usize, width: f32, height: f32 },
}

pub struct EventLoopHandle {
    system: Weak<RefCell<WaylandWindowingSystem>>,
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
        let loop_handle = system.borrow().event_loop_handle();

        loop_handle
            .insert_source(source, move |event, metadata, window_state| {
                let runtime_state = RuntimeState { window_state };
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
    window_state: &'a mut WindowState,
}

impl RuntimeState<'_> {
    #[must_use]
    pub fn component_instance(&self) -> &ComponentInstance {
        self.window_state.component_instance()
    }

    pub fn render_frame_if_dirty(&mut self) -> Result<()> {
        Ok(self.window_state.render_frame_if_dirty()?)
    }

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.window_state.compilation_result()
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

        let popup_manager = self
            .window_state
            .popup_manager()
            .as_ref()
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: "No popup manager available".to_string(),
                })
            })
            .map(Rc::clone)?;

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

        popup_manager.set_pending_popup(req, initial_dimensions.0, initial_dimensions.1);

        let instance = Self::create_popup_instance(&definition, &popup_manager, 0, resize_sender)?;

        let popup_key = popup_manager.current_popup_key().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No popup key available after creation".to_string(),
            })
        })?;

        if let Some(popup_window) = popup_manager.get_popup_window(popup_key) {
            popup_window.set_component_instance(instance);
        } else {
            return Err(Error::Domain(DomainError::Configuration {
                message: "Popup window not found after creation".to_string(),
            }));
        }

        Ok(PopupHandle::new(popup_key))
    }

    pub fn close_popup(&mut self, handle: PopupHandle) -> Result<()> {
        if let Some(popup_manager) = self.window_state.popup_manager() {
            popup_manager.destroy_popup(handle.key());
        }
        Ok(())
    }

    pub fn close_current_popup(&mut self) -> Result<()> {
        if let Some(popup_manager) = self.window_state.popup_manager() {
            popup_manager.close_current_popup();
        }
        Ok(())
    }

    pub fn resize_popup(
        &mut self,
        key: usize,
        width: f32,
        height: f32,
        resize_sender: Option<channel::Sender<PopupCommand>>,
    ) -> Result<()> {
        let popup_manager = self
            .window_state
            .popup_manager()
            .as_ref()
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: "No popup manager available".to_string(),
                })
            })
            .map(Rc::clone)?;

        let (request, _serial) = popup_manager.get_popup_info(key).ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: format!("Popup with key {} not found", key),
            })
        })?;

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

            self.close_popup(PopupHandle::new(key))?;

            let new_request = PopupRequest::builder(request.component)
                .at(request.at)
                .size(PopupSize::fixed(width, height))
                .mode(request.mode)
                .build();

            self.show_popup(new_request, resize_sender)?;
        } else if size_changed {
            if let Some(popup_window) = popup_manager.get_popup_window(key) {
                popup_window.request_resize(width, height);
            }
        }

        Ok(())
    }

    fn create_popup_instance(
        definition: &ComponentDefinition,
        popup_manager: &Rc<PopupManager>,
        popup_key: usize,
        resize_sender: Option<channel::Sender<PopupCommand>>,
    ) -> Result<ComponentInstance> {
        let instance = definition.create().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to create popup instance: {}", e),
            })
        })?;

        let popup_manager_weak = Rc::downgrade(popup_manager);
        instance
            .set_callback("closed", move |_| {
                if let Some(popup_manager) = popup_manager_weak.upgrade() {
                    popup_manager.close_current_popup();
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set popup closed callback: {}", e),
                })
            })?;

        let result = if let Some(sender) = resize_sender {
            instance.set_callback("change_popup_size", move |args| {
                let width: f32 = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(200.0);
                let height: f32 = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(150.0);

                log::info!("change_popup_size callback invoked: {}x{}", width, height);

                if sender
                    .send(PopupCommand::Resize {
                        key: popup_key,
                        width,
                        height,
                    })
                    .is_err()
                {
                    log::error!("Failed to send popup resize command through channel");
                }
                Value::Void
            })
        } else {
            let popup_manager_for_resize = Rc::downgrade(popup_manager);
            instance.set_callback("change_popup_size", move |args| {
                let width: f32 = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(200.0);
                let height: f32 = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(150.0);

                log::info!("change_popup_size callback invoked: {}x{}", width, height);

                if let Some(popup_window) = popup_manager_for_resize
                    .upgrade()
                    .and_then(|mgr| mgr.get_popup_window(popup_key))
                {
                    popup_window.request_resize(width, height);
                }
                Value::Void
            })
        };

        if let Err(e) = result {
            log::warn!("Failed to set change_popup_size callback: {}", e);
        } else {
            log::info!("change_popup_size callback registered successfully");
        }

        instance.show().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to show popup instance: {}", e),
            })
        })?;

        Ok(instance)
    }
}

pub struct WindowingSystem {
    inner: Rc<RefCell<WaylandWindowingSystem>>,
    popup_positioning_mode: Rc<RefCell<PopupPositioningMode>>,
    popup_command_sender: channel::Sender<PopupCommand>,
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
        let inner = WaylandWindowingSystem::new(wayland_config)?;
        let inner_rc = Rc::new(RefCell::new(inner));

        let (sender, receiver) = channel::channel();

        let system = Self {
            inner: Rc::clone(&inner_rc),
            popup_positioning_mode: Rc::new(RefCell::new(PopupPositioningMode::Center)),
            popup_command_sender: sender,
        };

        system.setup_popup_command_handler(receiver)?;
        system.register_popup_callbacks()?;

        Ok(system)
    }

    fn setup_popup_command_handler(&self, receiver: channel::Channel<PopupCommand>) -> Result<()> {
        let loop_handle = self.inner.borrow().event_loop_handle();
        let sender_for_handler = self.popup_command_sender.clone();

        loop_handle
            .insert_source(receiver, move |event, (), window_state| {
                if let channel::Event::Msg(command) = event {
                    let mut runtime_state = RuntimeState { window_state };

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
                        PopupCommand::Resize { key, width, height } => {
                            if let Err(e) = runtime_state.resize_popup(
                                key,
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

    fn register_popup_callbacks(&self) -> Result<()> {
        let component_instance = self.component_instance();

        let popup_mode_clone = Rc::clone(&self.popup_positioning_mode);
        component_instance
            .set_callback("set_popup_positioning_mode", move |args| {
                let center_x: bool = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(false);
                let center_y: bool = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(false);

                let mode = PopupPositioningMode::from_flags(center_x, center_y);
                *popup_mode_clone.borrow_mut() = mode;
                log::info!(
                    "Popup positioning mode set to: {:?} (center_x: {}, center_y: {})",
                    mode,
                    center_x,
                    center_y
                );
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "Failed to register set_popup_positioning_mode callback: {}",
                        e
                    ),
                })
            })?;

        let sender = self.popup_command_sender.clone();
        let popup_mode_for_callback = Rc::clone(&self.popup_positioning_mode);

        component_instance
            .set_callback("show_popup", move |args| {
                let component_name: SharedString = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or_else(|| SharedString::from(""));

                if component_name.is_empty() {
                    log::error!("show_popup called without component name");
                    return Value::Void;
                }

                let x: f32 = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);
                let y: f32 = args
                    .get(2)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);

                let mode = *popup_mode_for_callback.borrow();

                let request = PopupRequest::builder(component_name.to_string())
                    .at(PopupAt::absolute(x, y))
                    .size(PopupSize::content())
                    .mode(mode)
                    .build();

                if sender.send(PopupCommand::Show(request)).is_err() {
                    log::error!("Failed to send popup show command through channel");
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to register show_popup callback: {}", e),
                })
            })?;

        Ok(())
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

    #[must_use]
    pub fn component_instance(&self) -> Ref<'_, ComponentInstance> {
        Ref::map(self.inner.borrow(), |system| system.component_instance())
    }
}
