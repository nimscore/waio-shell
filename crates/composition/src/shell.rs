use crate::shell_composition::ShellWindowDefinition;
use crate::shell_runtime::ShellRuntime;
use crate::system::{EventContext, PopupCommand, ShellControl};
use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::{
    EventSource, Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer,
    channel,
};
use layer_shika_adapters::platform::slint_interpreter::{CompilationResult, ComponentInstance};
use layer_shika_adapters::platform::wayland::Anchor;
use layer_shika_adapters::{
    AppState, ShellWindowConfig, WaylandWindowConfig, WindowState, WindowingSystemFacade,
};
use layer_shika_domain::config::WindowConfig;
use layer_shika_domain::errors::DomainError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::unix::io::AsFd;
use std::rc::{Rc, Weak};
use std::result::Result as StdResult;
use std::time::{Duration, Instant};

pub struct LayerSurfaceHandle<'a> {
    window_state: &'a WindowState,
}

impl LayerSurfaceHandle<'_> {
    pub fn set_anchor(&self, anchor: Anchor) {
        self.window_state.layer_surface().set_anchor(anchor);
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.window_state.layer_surface().set_size(width, height);
    }

    pub fn set_exclusive_zone(&self, zone: i32) {
        self.window_state.layer_surface().set_exclusive_zone(zone);
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

impl Shell {
    pub(crate) fn new(
        compilation_result: Rc<CompilationResult>,
        definitions: &[ShellWindowDefinition],
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
                    .component(&def.component_name)
                    .ok_or_else(|| {
                        Error::Domain(DomainError::Configuration {
                            message: format!(
                                "Component '{}' not found in compilation result",
                                def.component_name
                            ),
                        })
                    })?;

                let wayland_config = WaylandWindowConfig::from_domain_config(
                    component_definition,
                    Some(Rc::clone(&compilation_result)),
                    def.config.clone(),
                );

                Ok(ShellWindowConfig {
                    name: def.component_name.clone(),
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
                def.component_name.clone(),
                ShellWindowHandle {
                    name: def.component_name.clone(),
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
        ShellEventLoopHandle {
            system: Rc::downgrade(&self.inner),
        }
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
}

impl ShellRuntime for Shell {
    type LoopHandle = ShellEventLoopHandle;
    type Context<'a> = ShellEventContext<'a>;

    fn event_loop_handle(&self) -> Self::LoopHandle {
        ShellEventLoopHandle {
            system: Rc::downgrade(&self.inner),
        }
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

pub struct ShellEventLoopHandle {
    system: Weak<RefCell<WindowingSystemFacade>>,
}

impl ShellEventLoopHandle {
    pub fn insert_source<S, F, R>(
        &self,
        source: S,
        mut callback: F,
    ) -> StdResult<RegistrationToken, Error>
    where
        S: EventSource<Ret = R> + 'static,
        F: FnMut(S::Event, &mut S::Metadata, ShellEventContext<'_>) -> R + 'static,
    {
        let system = self.system.upgrade().ok_or(Error::SystemDropped)?;
        let loop_handle = system.borrow().inner_ref().event_loop_handle();

        loop_handle
            .insert_source(source, move |event, metadata, app_state| {
                let ctx = ShellEventContext { app_state };
                callback(event, metadata, ctx)
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
        F: FnMut(Instant, ShellEventContext<'_>) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_duration(duration);
        self.insert_source(timer, move |deadline, (), ctx| callback(deadline, ctx))
    }

    pub fn add_channel<T, F>(
        &self,
        mut callback: F,
    ) -> Result<(RegistrationToken, channel::Sender<T>)>
    where
        T: 'static,
        F: FnMut(T, ShellEventContext<'_>) + 'static,
    {
        let (sender, receiver) = channel::channel();
        let token = self.insert_source(receiver, move |event, (), ctx| {
            if let channel::Event::Msg(msg) = event {
                callback(msg, ctx);
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
        F: FnMut(ShellEventContext<'_>) + 'static,
    {
        let generic = Generic::new(fd, interest, mode);
        self.insert_source(generic, move |_readiness, _fd, ctx| {
            callback(ctx);
            Ok(PostAction::Continue)
        })
    }
}

pub struct ShellEventContext<'a> {
    app_state: &'a mut AppState,
}

impl ShellEventContext<'_> {
    pub fn get_shell_window_component(
        &self,
        _shell_window_name: &str,
    ) -> Option<&ComponentInstance> {
        self.app_state
            .primary_output()
            .map(WindowState::component_instance)
    }

    pub fn get_shell_window_component_mut(
        &mut self,
        _shell_window_name: &str,
    ) -> Option<&ComponentInstance> {
        self.app_state
            .primary_output()
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
}
