use crate::event_loop::{EventLoopHandleBase, FromAppState};
use crate::shell_runtime::{DEFAULT_WINDOW_NAME, ShellRuntime};
use crate::value_conversion::IntoValue;
use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::channel;
use layer_shika_adapters::platform::slint::ComponentHandle;
use layer_shika_adapters::platform::slint_interpreter::{
    CompilationResult, ComponentDefinition, ComponentInstance, Value,
};
use layer_shika_adapters::{
    AppState, PopupManager, WaylandWindowConfig, WindowState, WindowingSystemFacade,
};
use layer_shika_domain::config::WindowConfig;
use layer_shika_domain::entities::output_registry::OutputRegistry;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::dimensions::PopupDimensions;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{
    PopupHandle, PopupPlacement, PopupRequest, PopupSize,
};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

pub enum PopupCommand {
    Show(PopupRequest),
    Close(PopupHandle),
    Resize {
        handle: PopupHandle,
        width: f32,
        height: f32,
    },
}

#[derive(Clone)]
pub struct ShellControl {
    sender: channel::Sender<PopupCommand>,
}

impl ShellControl {
    pub fn new(sender: channel::Sender<PopupCommand>) -> Self {
        Self { sender }
    }

    pub fn show_popup(&self, request: &PopupRequest) -> Result<()> {
        self.sender
            .send(PopupCommand::Show(request.clone()))
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup show command: channel closed".to_string(),
                })
            })
    }

    pub fn show_popup_at_cursor(&self, component: impl Into<String>) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtCursor)
            .build();
        self.show_popup(&request)
    }

    pub fn show_popup_centered(&self, component: impl Into<String>) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtCursor)
            .mode(PopupPositioningMode::Center)
            .build();
        self.show_popup(&request)
    }

    pub fn show_popup_at_position(
        &self,
        component: impl Into<String>,
        x: f32,
        y: f32,
    ) -> Result<()> {
        let request = PopupRequest::builder(component.into())
            .placement(PopupPlacement::AtPosition { x, y })
            .build();
        self.show_popup(&request)
    }

    pub fn close_popup(&self, handle: PopupHandle) -> Result<()> {
        self.sender.send(PopupCommand::Close(handle)).map_err(|_| {
            Error::Domain(DomainError::Configuration {
                message: "Failed to send popup close command: channel closed".to_string(),
            })
        })
    }

    pub fn resize_popup(&self, handle: PopupHandle, width: f32, height: f32) -> Result<()> {
        self.sender
            .send(PopupCommand::Resize {
                handle,
                width,
                height,
            })
            .map_err(|_| {
                Error::Domain(DomainError::Configuration {
                    message: "Failed to send popup resize command: channel closed".to_string(),
                })
            })
    }
}

pub type EventLoopHandle = EventLoopHandleBase<EventContext<'static>>;

pub struct EventContext<'a> {
    app_state: &'a mut AppState,
}

impl<'a> FromAppState<'a> for EventContext<'a> {
    fn from_app_state(app_state: &'a mut AppState) -> Self {
        Self { app_state }
    }
}

fn extract_dimensions_from_callback(args: &[Value]) -> PopupDimensions {
    let defaults = PopupDimensions::default();
    PopupDimensions::new(
        args.first()
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or(defaults.width),
        args.get(1)
            .and_then(|v| v.clone().try_into().ok())
            .unwrap_or(defaults.height),
    )
}

impl EventContext<'_> {
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

    pub const fn output_registry(&self) -> &OutputRegistry {
        self.app_state.output_registry()
    }

    #[must_use]
    pub fn primary_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.primary_output_handle()
    }

    #[must_use]
    pub fn active_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.active_output_handle()
    }

    pub fn outputs(&self) -> impl Iterator<Item = (OutputHandle, &ComponentInstance)> {
        self.app_state
            .outputs_with_handles()
            .map(|(handle, window)| (handle, window.component_instance()))
    }

    pub fn get_output_component(&self, handle: OutputHandle) -> Option<&ComponentInstance> {
        self.app_state
            .get_output_by_handle(handle)
            .map(WindowState::component_instance)
    }

    pub fn get_output_info(&self, handle: OutputHandle) -> Option<&OutputInfo> {
        self.app_state.get_output_info(handle)
    }

    pub fn all_output_info(&self) -> impl Iterator<Item = &OutputInfo> {
        self.app_state.all_output_info()
    }

    pub fn outputs_with_info(&self) -> impl Iterator<Item = (&OutputInfo, &ComponentInstance)> {
        self.app_state
            .outputs_with_info()
            .map(|(info, window)| (info, window.component_instance()))
    }

    fn active_or_primary_output(&self) -> Option<&WindowState> {
        self.app_state
            .active_output()
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
        req: &PopupRequest,
        resize_control: Option<ShellControl>,
    ) -> Result<PopupHandle> {
        log::info!("show_popup called for component '{}'", req.component);

        let compilation_result = self.compilation_result().ok_or_else(|| {
            log::error!("No compilation result available");
            Error::Domain(DomainError::Configuration {
                message: "No compilation result available for popup creation".to_string(),
            })
        })?;

        log::debug!(
            "Got compilation result, looking for component '{}'",
            req.component
        );

        let definition = compilation_result
            .component(&req.component)
            .ok_or_else(|| {
                log::error!(
                    "Component '{}' not found in compilation result",
                    req.component
                );
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "{} component not found in compilation result",
                        req.component
                    ),
                })
            })?;

        log::debug!("Found component definition for '{}'", req.component);

        self.close_current_popup()?;

        let is_using_active = self.app_state.active_output().is_some();
        let active_window = self.active_or_primary_output().ok_or_else(|| {
            log::error!("No active or primary output available");
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
            req.placement.position().0,
            req.placement.position().1,
            req.mode
        );

        let popup_handle =
            popup_manager.request_popup(req.clone(), initial_dimensions.0, initial_dimensions.1);

        let (instance, popup_key_cell) =
            Self::create_popup_instance(&definition, &popup_manager, resize_control, req)?;

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

    pub fn resize_popup(&mut self, handle: PopupHandle, width: f32, height: f32) -> Result<()> {
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

        if size_changed {
            if let Some(popup_window) = popup_manager.get_popup_window(handle.key()) {
                popup_window.request_resize(width, height);

                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_possible_wrap)]
                let logical_width = width as i32;
                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_possible_wrap)]
                let logical_height = height as i32;

                popup_manager.update_popup_viewport(handle.key(), logical_width, logical_height);
                popup_manager.commit_popup_surface(handle.key());
                log::debug!(
                    "Updated popup viewport to logical size: {}x{} (from resize to {}x{})",
                    logical_width,
                    logical_height,
                    width,
                    height
                );
            }
        }

        Ok(())
    }

    fn create_popup_instance(
        definition: &ComponentDefinition,
        popup_manager: &Rc<PopupManager>,
        resize_control: Option<ShellControl>,
        req: &PopupRequest,
    ) -> Result<(ComponentInstance, Rc<Cell<usize>>)> {
        let instance = definition.create().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to create popup instance: {}", e),
            })
        })?;

        let popup_key_cell = Rc::new(Cell::new(0));

        Self::register_popup_callbacks(
            &instance,
            popup_manager,
            resize_control,
            &popup_key_cell,
            req,
        )?;

        instance.show().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to show popup instance: {}", e),
            })
        })?;

        Ok((instance, popup_key_cell))
    }

    fn register_popup_callbacks(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        resize_control: Option<ShellControl>,
        popup_key_cell: &Rc<Cell<usize>>,
        req: &PopupRequest,
    ) -> Result<()> {
        if let Some(close_callback_name) = &req.close_callback {
            Self::register_close_callback(instance, popup_manager, close_callback_name)?;
        }

        if let Some(resize_callback_name) = &req.resize_callback {
            Self::register_resize_callback(
                instance,
                popup_manager,
                resize_control,
                popup_key_cell,
                resize_callback_name,
            )?;
        }

        Ok(())
    }

    fn register_close_callback(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        callback_name: &str,
    ) -> Result<()> {
        let popup_manager_weak = Rc::downgrade(popup_manager);
        instance
            .set_callback(callback_name, move |_| {
                if let Some(popup_manager) = popup_manager_weak.upgrade() {
                    popup_manager.close_current_popup();
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set '{}' callback: {}", callback_name, e),
                })
            })
    }

    fn register_resize_callback(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        resize_control: Option<ShellControl>,
        popup_key_cell: &Rc<Cell<usize>>,
        callback_name: &str,
    ) -> Result<()> {
        if let Some(control) = resize_control {
            Self::register_resize_with_control(instance, popup_key_cell, &control, callback_name)
        } else {
            Self::register_resize_direct(instance, popup_manager, popup_key_cell, callback_name)
        }
    }

    fn register_resize_with_control(
        instance: &ComponentInstance,
        popup_key_cell: &Rc<Cell<usize>>,
        control: &ShellControl,
        callback_name: &str,
    ) -> Result<()> {
        let key_cell = Rc::clone(popup_key_cell);
        let control = control.clone();
        instance
            .set_callback(callback_name, move |args| {
                let dimensions = extract_dimensions_from_callback(args);
                let popup_key = key_cell.get();

                log::info!(
                    "Resize callback invoked: {}x{} for key {}",
                    dimensions.width,
                    dimensions.height,
                    popup_key
                );

                if control
                    .resize_popup(
                        PopupHandle::new(popup_key),
                        dimensions.width,
                        dimensions.height,
                    )
                    .is_err()
                {
                    log::error!("Failed to resize popup through control");
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set '{}' callback: {}", callback_name, e),
                })
            })
    }

    fn register_resize_direct(
        instance: &ComponentInstance,
        popup_manager: &Rc<PopupManager>,
        popup_key_cell: &Rc<Cell<usize>>,
        callback_name: &str,
    ) -> Result<()> {
        let popup_manager_weak = Rc::downgrade(popup_manager);
        let key_cell = Rc::clone(popup_key_cell);
        instance
            .set_callback(callback_name, move |args| {
                let dimensions = extract_dimensions_from_callback(args);
                let popup_key = key_cell.get();

                log::info!(
                    "Resize callback invoked: {}x{} for key {}",
                    dimensions.width,
                    dimensions.height,
                    popup_key
                );

                if let Some(popup_manager) = popup_manager_weak.upgrade() {
                    if let Some(popup_window) = popup_manager.get_popup_window(popup_key) {
                        popup_window.request_resize(dimensions.width, dimensions.height);

                        #[allow(clippy::cast_possible_truncation)]
                        #[allow(clippy::cast_possible_wrap)]
                        let logical_width = dimensions.width as i32;
                        #[allow(clippy::cast_possible_truncation)]
                        #[allow(clippy::cast_possible_wrap)]
                        let logical_height = dimensions.height as i32;

                        popup_manager.update_popup_viewport(
                            popup_key,
                            logical_width,
                            logical_height,
                        );
                        log::debug!(
                            "Updated popup viewport to logical size: {}x{} (from direct resize to {}x{})",
                            logical_width,
                            logical_height,
                            dimensions.width,
                            dimensions.height
                        );
                    }
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set '{}' callback: {}", callback_name, e),
                })
            })
    }
}

pub struct SingleWindowShell {
    inner: Rc<RefCell<WindowingSystemFacade>>,
    popup_command_sender: channel::Sender<PopupCommand>,
    window_name: String,
}

#[allow(dead_code)]
impl SingleWindowShell {
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

        let shell = Self {
            inner: Rc::clone(&inner_rc),
            popup_command_sender: sender,
            window_name: DEFAULT_WINDOW_NAME.to_string(),
        };

        shell.setup_popup_command_handler(receiver)?;

        Ok(shell)
    }

    #[must_use]
    pub fn with_window_name(mut self, name: impl Into<String>) -> Self {
        self.window_name = name.into();
        self
    }

    #[must_use]
    pub fn window_name(&self) -> &str {
        &self.window_name
    }

    fn setup_popup_command_handler(&self, receiver: channel::Channel<PopupCommand>) -> Result<()> {
        let loop_handle = self.inner.borrow().inner_ref().event_loop_handle();
        let control = self.control();

        loop_handle
            .insert_source(receiver, move |event, (), app_state| {
                if let channel::Event::Msg(command) = event {
                    let mut shell_context = EventContext { app_state };

                    match command {
                        PopupCommand::Show(request) => {
                            if let Err(e) =
                                shell_context.show_popup(&request, Some(control.clone()))
                            {
                                log::error!("Failed to show popup: {}", e);
                            }
                        }
                        PopupCommand::Close(handle) => {
                            if let Err(e) = shell_context.close_popup(handle) {
                                log::error!("Failed to close popup: {}", e);
                            }
                        }
                        PopupCommand::Resize {
                            handle,
                            width,
                            height,
                        } => {
                            if let Err(e) = shell_context.resize_popup(handle, width, height) {
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

    #[must_use]
    pub fn control(&self) -> ShellControl {
        ShellControl {
            sender: self.popup_command_sender.clone(),
        }
    }

    #[must_use]
    pub fn event_loop_handle(&self) -> EventLoopHandle {
        EventLoopHandle::new(Rc::downgrade(&self.inner))
    }

    pub fn on<F, R>(&self, callback_name: &str, handler: F) -> Result<()>
    where
        F: Fn(ShellControl) -> R + 'static,
        R: IntoValue,
    {
        let control = self.control();
        let handler = Rc::new(handler);
        self.with_all_component_instances(|instance| {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = instance.set_callback(callback_name, move |_args| {
                handler_rc(control_clone.clone()).into_value()
            }) {
                log::error!(
                    "Failed to register callback '{}' on component: {}",
                    callback_name,
                    e
                );
            }
        });
        Ok(())
    }

    pub fn on_with_args<F, R>(&self, callback_name: &str, handler: F) -> Result<()>
    where
        F: Fn(&[Value], ShellControl) -> R + 'static,
        R: IntoValue,
    {
        let control = self.control();
        let handler = Rc::new(handler);
        self.with_all_component_instances(|instance| {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = instance.set_callback(callback_name, move |args| {
                handler_rc(args, control_clone.clone()).into_value()
            }) {
                log::error!(
                    "Failed to register callback '{}' on component: {}",
                    callback_name,
                    e
                );
            }
        });
        Ok(())
    }

    pub fn on_for_output<F, R>(
        &self,
        output: OutputHandle,
        callback_name: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(ShellControl) -> R + 'static,
        R: IntoValue,
    {
        let control = self.control();
        self.with_output(output, |instance| {
            let control_clone = control.clone();
            if let Err(e) = instance.set_callback(callback_name, move |_args| {
                handler(control_clone.clone()).into_value()
            }) {
                log::error!(
                    "Failed to register callback '{}' on output {:?}: {}",
                    callback_name,
                    output,
                    e
                );
            }
        })?;
        Ok(())
    }

    pub fn on_for_output_with_args<F, R>(
        &self,
        output: OutputHandle,
        callback_name: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&[Value], ShellControl) -> R + 'static,
        R: IntoValue,
    {
        let control = self.control();
        self.with_output(output, |instance| {
            let control_clone = control.clone();
            if let Err(e) = instance.set_callback(callback_name, move |args| {
                handler(args, control_clone.clone()).into_value()
            }) {
                log::error!(
                    "Failed to register callback '{}' on output {:?}: {}",
                    callback_name,
                    output,
                    e
                );
            }
        })?;
        Ok(())
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

    pub fn with_output<F, R>(&self, handle: OutputHandle, f: F) -> Result<R>
    where
        F: FnOnce(&ComponentInstance) -> R,
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        let window = system
            .app_state()
            .get_output_by_handle(handle)
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Output with handle {:?} not found", handle),
                })
            })?;
        Ok(f(window.component_instance()))
    }

    pub fn with_all_outputs<F>(&self, mut f: F)
    where
        F: FnMut(OutputHandle, &ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        for (handle, window) in system.app_state().outputs_with_handles() {
            f(handle, window.component_instance());
        }
    }

    pub fn get_output_info(&self, handle: OutputHandle) -> Option<OutputInfo> {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        system.app_state().get_output_info(handle).cloned()
    }

    pub fn all_output_info(&self) -> Vec<OutputInfo> {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        system.app_state().all_output_info().cloned().collect()
    }

    pub fn output_registry(&self) -> OutputRegistry {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        system.app_state().output_registry().clone()
    }
}

impl ShellRuntime for SingleWindowShell {
    type LoopHandle = EventLoopHandle;
    type Context<'a> = EventContext<'a>;

    fn event_loop_handle(&self) -> Self::LoopHandle {
        EventLoopHandle::new(Rc::downgrade(&self.inner))
    }

    fn with_component<F>(&self, _name: &str, mut f: F)
    where
        F: FnMut(&ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        for window in system.app_state().all_outputs() {
            f(window.component_instance());
        }
    }

    fn with_all_components<F>(&self, mut f: F)
    where
        F: FnMut(&str, &ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        for window in system.app_state().all_outputs() {
            f(&self.window_name, window.component_instance());
        }
    }

    fn run(&mut self) -> Result<()> {
        self.inner.borrow_mut().run()?;
        Ok(())
    }
}
