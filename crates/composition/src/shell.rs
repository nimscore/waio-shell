use crate::event_loop::{EventLoopHandleBase, FromAppState};
use crate::layer_shika::WindowDefinition;
use crate::shell_runtime::ShellRuntime;
use crate::system::{EventContext, PopupCommand, ShellControl};
use crate::value_conversion::IntoValue;
use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::channel;
use layer_shika_adapters::platform::slint_interpreter::{
    CompilationResult, ComponentInstance, Value,
};
use layer_shika_adapters::platform::wayland::{Anchor, WaylandKeyboardInteractivity, WaylandLayer};
use layer_shika_adapters::{
    AppState, ShellWindowConfig, WaylandWindowConfig, WindowState, WindowingSystemFacade,
};
use layer_shika_domain::config::WindowConfig;
use layer_shika_domain::entities::output_registry::OutputRegistry;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::keyboard_interactivity::KeyboardInteractivity;
use layer_shika_domain::value_objects::layer::Layer;
use layer_shika_domain::value_objects::margins::Margins;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct LayerSurfaceHandle<'a> {
    window_state: &'a WindowState,
}

impl<'a> LayerSurfaceHandle<'a> {
    pub(crate) fn from_window_state(window_state: &'a WindowState) -> Self {
        Self { window_state }
    }

    pub fn set_anchor(&self, anchor: Anchor) {
        self.window_state.layer_surface().set_anchor(anchor);
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.window_state.layer_surface().set_size(width, height);
    }

    pub fn set_exclusive_zone(&self, zone: i32) {
        self.window_state.layer_surface().set_exclusive_zone(zone);
    }

    pub fn set_margins(&self, margins: Margins) {
        self.window_state.layer_surface().set_margin(
            margins.top,
            margins.right,
            margins.bottom,
            margins.left,
        );
    }

    pub fn set_keyboard_interactivity(&self, mode: KeyboardInteractivity) {
        let wayland_mode = match mode {
            KeyboardInteractivity::None => WaylandKeyboardInteractivity::None,
            KeyboardInteractivity::Exclusive => WaylandKeyboardInteractivity::Exclusive,
            KeyboardInteractivity::OnDemand => WaylandKeyboardInteractivity::OnDemand,
        };
        self.window_state
            .layer_surface()
            .set_keyboard_interactivity(wayland_mode);
    }

    pub fn set_layer(&self, layer: Layer) {
        let wayland_layer = match layer {
            Layer::Background => WaylandLayer::Background,
            Layer::Bottom => WaylandLayer::Bottom,
            Layer::Top => WaylandLayer::Top,
            Layer::Overlay => WaylandLayer::Overlay,
        };
        self.window_state.layer_surface().set_layer(wayland_layer);
    }

    pub fn commit(&self) {
        self.window_state.commit_surface();
    }
}

pub trait ShellWindowConfigHandler {
    fn configure_window(&self, instance: &ComponentInstance, surface: LayerSurfaceHandle<'_>);
}

impl<F> ShellWindowConfigHandler for F
where
    F: Fn(&ComponentInstance, LayerSurfaceHandle<'_>),
{
    fn configure_window(&self, instance: &ComponentInstance, surface: LayerSurfaceHandle<'_>) {
        self(instance, surface);
    }
}

#[derive(Debug, Clone)]
pub struct ShellWindowHandle {
    pub name: String,
}

pub struct Shell {
    inner: Rc<RefCell<WindowingSystemFacade>>,
    windows: HashMap<String, ShellWindowHandle>,
    compilation_result: Rc<CompilationResult>,
    popup_command_sender: channel::Sender<PopupCommand>,
}

#[allow(dead_code)]
impl Shell {
    pub(crate) fn new(
        compilation_result: Rc<CompilationResult>,
        definitions: &[WindowDefinition],
    ) -> Result<Self> {
        log::info!("Creating shell with {} windows", definitions.len());

        if definitions.is_empty() {
            return Err(Error::Domain(DomainError::Configuration {
                message: "At least one shell window definition is required".to_string(),
            }));
        }

        let shell_configs: Vec<ShellWindowConfig> = definitions
            .iter()
            .map(|def| {
                let component_definition = compilation_result
                    .component(&def.component)
                    .ok_or_else(|| {
                        Error::Domain(DomainError::Configuration {
                            message: format!(
                                "Component '{}' not found in compilation result",
                                def.component
                            ),
                        })
                    })?;

                let wayland_config = WaylandWindowConfig::from_domain_config(
                    component_definition,
                    Some(Rc::clone(&compilation_result)),
                    def.config.clone(),
                );

                Ok(ShellWindowConfig {
                    name: def.component.clone(),
                    config: wayland_config,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let inner = layer_shika_adapters::WaylandWindowingSystem::new_multi(&shell_configs)?;
        let facade = WindowingSystemFacade::new(inner);
        let inner_rc = Rc::new(RefCell::new(facade));

        let (sender, receiver) = channel::channel();

        let mut windows = HashMap::new();
        for def in definitions {
            windows.insert(
                def.component.clone(),
                ShellWindowHandle {
                    name: def.component.clone(),
                },
            );
        }

        let shell = Self {
            inner: Rc::clone(&inner_rc),
            windows,
            compilation_result,
            popup_command_sender: sender,
        };

        shell.setup_popup_command_handler(receiver)?;

        log::info!(
            "Shell created with windows: {:?}",
            shell.shell_window_names()
        );

        Ok(shell)
    }

    pub(crate) fn new_auto_discover(
        compilation_result: Rc<CompilationResult>,
        component_names: &[String],
    ) -> Result<Self> {
        log::info!(
            "Creating shell with auto-discovery for {} components",
            component_names.len()
        );

        if component_names.is_empty() {
            return Err(Error::Domain(DomainError::Configuration {
                message: "At least one component name is required for auto-discovery".to_string(),
            }));
        }

        let default_config = WindowConfig::default();

        let shell_configs: Vec<ShellWindowConfig> = component_names
            .iter()
            .map(|name| {
                let component_definition = compilation_result.component(name).ok_or_else(|| {
                    Error::Domain(DomainError::Configuration {
                        message: format!("Component '{}' not found in compilation result", name),
                    })
                })?;

                let wayland_config = WaylandWindowConfig::from_domain_config(
                    component_definition,
                    Some(Rc::clone(&compilation_result)),
                    default_config.clone(),
                );

                Ok(ShellWindowConfig {
                    name: name.clone(),
                    config: wayland_config,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let inner = layer_shika_adapters::WaylandWindowingSystem::new_multi(&shell_configs)?;
        let facade = WindowingSystemFacade::new(inner);
        let inner_rc = Rc::new(RefCell::new(facade));

        let (sender, receiver) = channel::channel();

        let mut windows = HashMap::new();
        for name in component_names {
            windows.insert(name.clone(), ShellWindowHandle { name: name.clone() });
        }

        let shell = Self {
            inner: Rc::clone(&inner_rc),
            windows,
            compilation_result,
            popup_command_sender: sender,
        };

        shell.setup_popup_command_handler(receiver)?;

        log::info!(
            "Shell created with auto-discovered windows: {:?}",
            shell.shell_window_names()
        );

        Ok(shell)
    }

    pub fn apply_window_config<H: ShellWindowConfigHandler>(&self, handler: &H) {
        log::info!("Applying window configuration via handler");

        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for window in system.app_state().all_outputs() {
            let instance = window.component_instance();
            let surface_handle = LayerSurfaceHandle {
                window_state: window,
            };
            handler.configure_window(instance, surface_handle);
        }
    }

    pub fn apply_window_config_fn<F>(&self, f: F)
    where
        F: Fn(&ComponentInstance, LayerSurfaceHandle<'_>),
    {
        self.apply_window_config(&f);
    }

    fn setup_popup_command_handler(&self, receiver: channel::Channel<PopupCommand>) -> Result<()> {
        let loop_handle = self.inner.borrow().inner_ref().event_loop_handle();
        let control = self.control();

        loop_handle
            .insert_source(receiver, move |event, (), app_state| {
                if let channel::Event::Msg(command) = event {
                    let mut ctx = EventContext::from_app_state(app_state);

                    match command {
                        PopupCommand::Show(request) => {
                            if let Err(e) = ctx.show_popup(&request, Some(control.clone())) {
                                log::error!("Failed to show popup: {}", e);
                            }
                        }
                        PopupCommand::Close(handle) => {
                            if let Err(e) = ctx.close_popup(handle) {
                                log::error!("Failed to close popup: {}", e);
                            }
                        }
                        PopupCommand::Resize {
                            handle,
                            width,
                            height,
                        } => {
                            if let Err(e) = ctx.resize_popup(handle, width, height) {
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
        ShellControl::new(self.popup_command_sender.clone())
    }

    pub fn shell_window(&self, name: &str) -> Option<&ShellWindowHandle> {
        self.windows.get(name)
    }

    pub fn shell_window_names(&self) -> Vec<&str> {
        self.windows.keys().map(String::as_str).collect()
    }

    pub fn event_loop_handle(&self) -> ShellEventLoopHandle {
        ShellEventLoopHandle::new(Rc::downgrade(&self.inner))
    }

    pub fn run(&mut self) -> Result<()> {
        log::info!(
            "Starting shell event loop with {} windows",
            self.windows.len()
        );
        self.inner.borrow_mut().run()?;
        Ok(())
    }

    pub fn with_component<F>(&self, shell_window_name: &str, mut f: F)
    where
        F: FnMut(&ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        if self.windows.contains_key(shell_window_name) {
            for window in system.app_state().windows_by_shell_name(shell_window_name) {
                f(window.component_instance());
            }
        }
    }

    pub fn with_all_components<F>(&self, mut f: F)
    where
        F: FnMut(&str, &ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for name in self.windows.keys() {
            if let Some(window) = system.app_state().primary_output() {
                f(name, window.component_instance());
            }
        }
    }

    #[must_use]
    pub fn compilation_result(&self) -> &Rc<CompilationResult> {
        &self.compilation_result
    }

    pub fn on<F, R>(&self, shell_window_name: &str, callback_name: &str, handler: F) -> Result<()>
    where
        F: Fn(ShellControl) -> R + 'static,
        R: IntoValue,
    {
        if !self.windows.contains_key(shell_window_name) {
            return Err(Error::Domain(DomainError::Configuration {
                message: format!("Shell window '{}' not found", shell_window_name),
            }));
        }

        let control = self.control();
        let handler = Rc::new(handler);
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for window in system.app_state().windows_by_shell_name(shell_window_name) {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = window
                .component_instance()
                .set_callback(callback_name, move |_args| {
                    handler_rc(control_clone.clone()).into_value()
                })
            {
                log::error!(
                    "Failed to register callback '{}' on window '{}': {}",
                    callback_name,
                    shell_window_name,
                    e
                );
            }
        }

        Ok(())
    }

    pub fn on_with_args<F, R>(
        &self,
        shell_window_name: &str,
        callback_name: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&[Value], ShellControl) -> R + 'static,
        R: IntoValue,
    {
        if !self.windows.contains_key(shell_window_name) {
            return Err(Error::Domain(DomainError::Configuration {
                message: format!("Shell window '{}' not found", shell_window_name),
            }));
        }

        let control = self.control();
        let handler = Rc::new(handler);
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for window in system.app_state().windows_by_shell_name(shell_window_name) {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = window
                .component_instance()
                .set_callback(callback_name, move |args| {
                    handler_rc(args, control_clone.clone()).into_value()
                })
            {
                log::error!(
                    "Failed to register callback '{}' on window '{}': {}",
                    callback_name,
                    shell_window_name,
                    e
                );
            }
        }

        Ok(())
    }

    pub fn on_global<F, R>(&self, callback_name: &str, handler: F) -> Result<()>
    where
        F: Fn(ShellControl) -> R + 'static,
        R: IntoValue,
    {
        let control = self.control();
        let handler = Rc::new(handler);
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for window in system.app_state().all_outputs() {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = window
                .component_instance()
                .set_callback(callback_name, move |_args| {
                    handler_rc(control_clone.clone()).into_value()
                })
            {
                log::error!(
                    "Failed to register global callback '{}': {}",
                    callback_name,
                    e
                );
            }
        }

        Ok(())
    }

    pub fn on_global_with_args<F, R>(&self, callback_name: &str, handler: F) -> Result<()>
    where
        F: Fn(&[Value], ShellControl) -> R + 'static,
        R: IntoValue,
    {
        let control = self.control();
        let handler = Rc::new(handler);
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for window in system.app_state().all_outputs() {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = window
                .component_instance()
                .set_callback(callback_name, move |args| {
                    handler_rc(args, control_clone.clone()).into_value()
                })
            {
                log::error!(
                    "Failed to register global callback '{}': {}",
                    callback_name,
                    e
                );
            }
        }

        Ok(())
    }
}

impl ShellRuntime for Shell {
    type LoopHandle = ShellEventLoopHandle;
    type Context<'a> = ShellEventContext<'a>;

    fn event_loop_handle(&self) -> Self::LoopHandle {
        ShellEventLoopHandle::new(Rc::downgrade(&self.inner))
    }

    fn with_component<F>(&self, name: &str, mut f: F)
    where
        F: FnMut(&ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        if self.windows.contains_key(name) {
            for window in system.app_state().windows_by_shell_name(name) {
                f(window.component_instance());
            }
        }
    }

    fn with_all_components<F>(&self, mut f: F)
    where
        F: FnMut(&str, &ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for name in self.windows.keys() {
            if let Some(window) = system.app_state().primary_output() {
                f(name, window.component_instance());
            }
        }
    }

    fn run(&mut self) -> Result<()> {
        log::info!(
            "Starting shell event loop with {} windows",
            self.windows.len()
        );
        self.inner.borrow_mut().run()?;
        Ok(())
    }
}

pub type ShellEventLoopHandle = EventLoopHandleBase<ShellEventContext<'static>>;

pub struct ShellEventContext<'a> {
    app_state: &'a mut AppState,
}

impl<'a> FromAppState<'a> for ShellEventContext<'a> {
    fn from_app_state(app_state: &'a mut AppState) -> Self {
        Self { app_state }
    }
}

impl ShellEventContext<'_> {
    pub fn get_shell_window_component(
        &self,
        shell_window_name: &str,
    ) -> Option<&ComponentInstance> {
        self.app_state
            .windows_by_shell_name(shell_window_name)
            .next()
            .map(WindowState::component_instance)
    }

    pub fn get_shell_window_component_mut(
        &mut self,
        shell_window_name: &str,
    ) -> Option<&ComponentInstance> {
        self.app_state
            .windows_by_shell_name(shell_window_name)
            .next()
            .map(WindowState::component_instance)
    }

    pub fn all_shell_window_components(&self) -> impl Iterator<Item = &ComponentInstance> {
        self.app_state
            .all_outputs()
            .map(WindowState::component_instance)
    }

    pub fn render_frame_if_dirty(&mut self) -> Result<()> {
        for window in self.app_state.all_outputs() {
            window.render_frame_if_dirty()?;
        }
        Ok(())
    }

    #[must_use]
    pub fn primary_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.primary_output_handle()
    }

    #[must_use]
    pub fn active_output_handle(&self) -> Option<OutputHandle> {
        self.app_state.active_output_handle()
    }

    pub fn output_registry(&self) -> &OutputRegistry {
        self.app_state.output_registry()
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

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.app_state
            .primary_output()
            .and_then(WindowState::compilation_result)
    }
}
