use crate::event_loop::{EventLoopHandleBase, FromAppState};
use crate::layer_surface::LayerSurfaceHandle;
use crate::popup_builder::PopupBuilder;
use crate::shell_config::{CompiledUiSource, ShellConfig};
use crate::shell_runtime::ShellRuntime;
use crate::system::{PopupCommand, ShellControl};
use crate::value_conversion::IntoValue;
use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::channel;
use layer_shika_adapters::platform::slint_interpreter::{
    CompilationResult, Compiler, ComponentInstance, Value,
};
use layer_shika_adapters::{
    AppState, ShellSurfaceConfig, ShellSystemFacade, SurfaceState, WaylandSurfaceConfig,
};
use layer_shika_domain::config::SurfaceConfig;
use layer_shika_domain::entities::output_registry::OutputRegistry;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::prelude::{
    AnchorEdges, KeyboardInteractivity, Layer, Margins, OutputPolicy, ScaleFactor, SurfaceDimension,
};
use layer_shika_domain::value_objects::handle::SurfaceHandle;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use spin_on::spin_on;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub const DEFAULT_COMPONENT_NAME: &str = "Main";

#[derive(Debug, Clone)]
pub struct SurfaceDefinition {
    pub component: String,
    pub config: SurfaceConfig,
}

enum CompilationSource {
    File { path: PathBuf, compiler: Compiler },
    Source { code: String, compiler: Compiler },
    Compiled(Rc<CompilationResult>),
}

pub struct ShellBuilder {
    compilation: CompilationSource,
    surfaces: Vec<SurfaceDefinition>,
}

impl ShellBuilder {
    pub fn surface(self, component: impl Into<String>) -> SurfaceConfigBuilder {
        SurfaceConfigBuilder {
            shell_builder: self,
            component: component.into(),
            config: SurfaceConfig::default(),
        }
    }

    #[must_use]
    pub fn discover_surfaces(
        mut self,
        components: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        for component in components {
            self.surfaces.push(SurfaceDefinition {
                component: component.into(),
                config: SurfaceConfig::default(),
            });
        }
        self
    }

    pub fn build(self) -> Result<Shell> {
        let surfaces = if self.surfaces.is_empty() {
            vec![SurfaceDefinition {
                component: DEFAULT_COMPONENT_NAME.to_string(),
                config: SurfaceConfig::default(),
            }]
        } else {
            self.surfaces
        };

        let compilation_result = match self.compilation {
            CompilationSource::File { path, compiler } => {
                let result = spin_on(compiler.build_from_path(&path));
                let diagnostics: Vec<_> = result.diagnostics().collect();
                if !diagnostics.is_empty() {
                    let messages: Vec<String> =
                        diagnostics.iter().map(ToString::to_string).collect();
                    return Err(DomainError::Configuration {
                        message: format!(
                            "Failed to compile Slint file '{}':\n{}",
                            path.display(),
                            messages.join("\n")
                        ),
                    }
                    .into());
                }
                Rc::new(result)
            }
            CompilationSource::Source { code, compiler } => {
                let result = spin_on(compiler.build_from_source(code, PathBuf::default()));
                let diagnostics: Vec<_> = result.diagnostics().collect();
                if !diagnostics.is_empty() {
                    let messages: Vec<String> =
                        diagnostics.iter().map(ToString::to_string).collect();
                    return Err(DomainError::Configuration {
                        message: format!(
                            "Failed to compile Slint source:\n{}",
                            messages.join("\n")
                        ),
                    }
                    .into());
                }
                Rc::new(result)
            }
            CompilationSource::Compiled(result) => result,
        };

        Shell::new(compilation_result, surfaces)
    }
}

pub struct SurfaceConfigBuilder {
    shell_builder: ShellBuilder,
    component: String,
    config: SurfaceConfig,
}

impl SurfaceConfigBuilder {
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.dimensions = SurfaceDimension::new(width, height);
        self
    }

    #[must_use]
    pub fn height(mut self, height: u32) -> Self {
        self.config.dimensions = SurfaceDimension::new(self.config.dimensions.width(), height);
        self
    }

    #[must_use]
    pub fn width(mut self, width: u32) -> Self {
        self.config.dimensions = SurfaceDimension::new(width, self.config.dimensions.height());
        self
    }

    #[must_use]
    pub const fn layer(mut self, layer: Layer) -> Self {
        self.config.layer = layer;
        self
    }

    #[must_use]
    pub fn margin(mut self, margin: impl Into<Margins>) -> Self {
        self.config.margin = margin.into();
        self
    }

    #[must_use]
    pub const fn anchor(mut self, anchor: AnchorEdges) -> Self {
        self.config.anchor = anchor;
        self
    }

    #[must_use]
    pub const fn exclusive_zone(mut self, zone: i32) -> Self {
        self.config.exclusive_zone = zone;
        self
    }

    #[must_use]
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.config.namespace = namespace.into();
        self
    }

    #[must_use]
    pub fn scale_factor(mut self, sf: impl TryInto<ScaleFactor, Error = DomainError>) -> Self {
        self.config.scale_factor = sf.try_into().unwrap_or_default();
        self
    }

    #[must_use]
    pub const fn keyboard_interactivity(mut self, mode: KeyboardInteractivity) -> Self {
        self.config.keyboard_interactivity = mode;
        self
    }

    #[must_use]
    pub fn output_policy(mut self, policy: OutputPolicy) -> Self {
        self.config.output_policy = policy;
        self
    }

    #[must_use]
    pub fn surface(self, component: impl Into<String>) -> SurfaceConfigBuilder {
        let shell_builder = self.complete();
        shell_builder.surface(component)
    }

    pub fn build(self) -> Result<Shell> {
        self.complete().build()
    }

    pub fn run(self) -> Result<()> {
        let mut shell = self.build()?;
        shell.run()
    }

    fn complete(mut self) -> ShellBuilder {
        self.shell_builder.surfaces.push(SurfaceDefinition {
            component: self.component,
            config: self.config,
        });
        self.shell_builder
    }
}

type OutputConnectedHandler = Box<dyn Fn(&OutputInfo)>;
type OutputDisconnectedHandler = Box<dyn Fn(OutputHandle)>;

pub struct Shell {
    inner: Rc<RefCell<ShellSystemFacade>>,
    surfaces: HashMap<String, SurfaceDefinition>,
    surface_handles: HashMap<SurfaceHandle, String>,
    compilation_result: Rc<CompilationResult>,
    popup_command_sender: channel::Sender<PopupCommand>,
    output_connected_handlers: Rc<RefCell<Vec<OutputConnectedHandler>>>,
    output_disconnected_handlers: Rc<RefCell<Vec<OutputDisconnectedHandler>>>,
}

impl Shell {
    pub fn from_file(path: impl AsRef<Path>) -> ShellBuilder {
        ShellBuilder {
            compilation: CompilationSource::File {
                path: path.as_ref().to_path_buf(),
                compiler: Compiler::default(),
            },
            surfaces: Vec::new(),
        }
    }

    pub fn from_file_with_compiler(path: impl AsRef<Path>, compiler: Compiler) -> ShellBuilder {
        ShellBuilder {
            compilation: CompilationSource::File {
                path: path.as_ref().to_path_buf(),
                compiler,
            },
            surfaces: Vec::new(),
        }
    }

    pub fn from_source(code: impl Into<String>) -> ShellBuilder {
        ShellBuilder {
            compilation: CompilationSource::Source {
                code: code.into(),
                compiler: Compiler::default(),
            },
            surfaces: Vec::new(),
        }
    }

    pub fn from_source_with_compiler(code: impl Into<String>, compiler: Compiler) -> ShellBuilder {
        ShellBuilder {
            compilation: CompilationSource::Source {
                code: code.into(),
                compiler,
            },
            surfaces: Vec::new(),
        }
    }

    pub fn from_compilation(result: Rc<CompilationResult>) -> ShellBuilder {
        ShellBuilder {
            compilation: CompilationSource::Compiled(result),
            surfaces: Vec::new(),
        }
    }

    pub fn builder() -> ShellBuilder {
        ShellBuilder {
            compilation: CompilationSource::Source {
                code: String::new(),
                compiler: Compiler::default(),
            },
            surfaces: Vec::new(),
        }
    }

    pub fn compile_file(path: impl AsRef<Path>) -> Result<Rc<CompilationResult>> {
        let compiler = Compiler::default();
        let result = spin_on(compiler.build_from_path(path.as_ref()));
        let diagnostics: Vec<_> = result.diagnostics().collect();
        if !diagnostics.is_empty() {
            let messages: Vec<String> = diagnostics.iter().map(ToString::to_string).collect();
            return Err(DomainError::Configuration {
                message: format!(
                    "Failed to compile Slint file '{}':\n{}",
                    path.as_ref().display(),
                    messages.join("\n")
                ),
            }
            .into());
        }
        Ok(Rc::new(result))
    }

    pub fn compile_source(code: impl Into<String>) -> Result<Rc<CompilationResult>> {
        let compiler = Compiler::default();
        let result = spin_on(compiler.build_from_source(code.into(), PathBuf::default()));
        let diagnostics: Vec<_> = result.diagnostics().collect();
        if !diagnostics.is_empty() {
            let messages: Vec<String> = diagnostics.iter().map(ToString::to_string).collect();
            return Err(DomainError::Configuration {
                message: format!("Failed to compile Slint source:\n{}", messages.join("\n")),
            }
            .into());
        }
        Ok(Rc::new(result))
    }

    pub fn from_config(config: ShellConfig) -> Result<Self> {
        let compilation_result = match config.ui_source {
            CompiledUiSource::File(path) => Self::compile_file(&path)?,
            CompiledUiSource::Source(code) => Self::compile_source(code)?,
            CompiledUiSource::Compiled(result) => result,
        };

        let surfaces: Vec<SurfaceDefinition> = if config.surfaces.is_empty() {
            vec![SurfaceDefinition {
                component: DEFAULT_COMPONENT_NAME.to_string(),
                config: SurfaceConfig::default(),
            }]
        } else {
            config
                .surfaces
                .into_iter()
                .map(|s| SurfaceDefinition {
                    component: s.component,
                    config: s.config,
                })
                .collect()
        };

        Self::new(compilation_result, surfaces)
    }

    pub(crate) fn new(
        compilation_result: Rc<CompilationResult>,
        definitions: Vec<SurfaceDefinition>,
    ) -> Result<Self> {
        log::info!("Creating Shell with {} windows", definitions.len());

        if definitions.is_empty() {
            return Err(Error::Domain(DomainError::Configuration {
                message: "At least one window definition is required".to_string(),
            }));
        }

        let is_single_window = definitions.len() == 1;

        if is_single_window {
            let definition = definitions.into_iter().next().ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: "Expected at least one window definition".to_string(),
                })
            })?;
            Self::new_single_window(compilation_result, definition)
        } else {
            Self::new_multi_window(compilation_result, definitions)
        }
    }

    fn new_single_window(
        compilation_result: Rc<CompilationResult>,
        definition: SurfaceDefinition,
    ) -> Result<Self> {
        let component_definition = compilation_result
            .component(&definition.component)
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "Component '{}' not found in compilation result",
                        definition.component
                    ),
                })
            })?;

        let wayland_config = WaylandSurfaceConfig::from_domain_config(
            &definition.component,
            component_definition,
            Some(Rc::clone(&compilation_result)),
            definition.config.clone(),
        );

        let inner = layer_shika_adapters::WaylandShellSystem::new(&wayland_config)?;
        let facade = ShellSystemFacade::new(inner);
        let inner_rc = Rc::new(RefCell::new(facade));

        let (sender, receiver) = channel::channel();

        let mut surfaces = HashMap::new();
        let mut surface_handles = HashMap::new();
        let handle = SurfaceHandle::new();
        surface_handles.insert(handle, definition.component.clone());
        surfaces.insert(definition.component.clone(), definition);

        let shell = Self {
            inner: Rc::clone(&inner_rc),
            surfaces,
            surface_handles,
            compilation_result,
            popup_command_sender: sender,
            output_connected_handlers: Rc::new(RefCell::new(Vec::new())),
            output_disconnected_handlers: Rc::new(RefCell::new(Vec::new())),
        };

        shell.setup_popup_command_handler(receiver)?;

        log::info!("Shell created (single-window mode)");

        Ok(shell)
    }

    fn new_multi_window(
        compilation_result: Rc<CompilationResult>,
        definitions: Vec<SurfaceDefinition>,
    ) -> Result<Self> {
        let shell_configs: Vec<ShellSurfaceConfig> = definitions
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

                let wayland_config = WaylandSurfaceConfig::from_domain_config(
                    &def.component,
                    component_definition,
                    Some(Rc::clone(&compilation_result)),
                    def.config.clone(),
                );

                Ok(ShellSurfaceConfig {
                    name: def.component.clone(),
                    config: wayland_config,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let inner = layer_shika_adapters::WaylandShellSystem::new_multi(&shell_configs)?;
        let facade = ShellSystemFacade::new(inner);
        let inner_rc = Rc::new(RefCell::new(facade));

        let (sender, receiver) = channel::channel();

        let mut surfaces = HashMap::new();
        let mut surface_handles = HashMap::new();
        for definition in definitions {
            let handle = SurfaceHandle::new();
            surface_handles.insert(handle, definition.component.clone());
            surfaces.insert(definition.component.clone(), definition);
        }

        let shell = Self {
            inner: Rc::clone(&inner_rc),
            surfaces,
            surface_handles,
            compilation_result,
            popup_command_sender: sender,
            output_connected_handlers: Rc::new(RefCell::new(Vec::new())),
            output_disconnected_handlers: Rc::new(RefCell::new(Vec::new())),
        };

        shell.setup_popup_command_handler(receiver)?;

        log::info!(
            "Shell created (multi-surface mode) with surfaces: {:?}",
            shell.surface_names()
        );

        Ok(shell)
    }

    fn setup_popup_command_handler(&self, receiver: channel::Channel<PopupCommand>) -> Result<()> {
        let loop_handle = self.inner.borrow().inner_ref().event_loop_handle();
        let control = self.control();

        loop_handle
            .insert_source(receiver, move |event, (), app_state| {
                if let channel::Event::Msg(command) = event {
                    let mut ctx = crate::system::EventContext::from_app_state(app_state);

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

    pub fn surface_names(&self) -> Vec<&str> {
        self.surfaces.keys().map(String::as_str).collect()
    }

    pub fn has_surface(&self, name: &str) -> bool {
        self.surfaces.contains_key(name)
    }

    pub fn event_loop_handle(&self) -> ShellEventLoopHandle {
        ShellEventLoopHandle::new(Rc::downgrade(&self.inner))
    }

    pub fn run(&mut self) -> Result<()> {
        log::info!(
            "Starting Shell event loop with {} windows",
            self.surfaces.len()
        );
        self.inner.borrow_mut().run()?;
        Ok(())
    }

    pub fn spawn_surface(&mut self, definition: SurfaceDefinition) -> Result<Vec<SurfaceHandle>> {
        let component_definition = self
            .compilation_result
            .component(&definition.component)
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "Component '{}' not found in compilation result",
                        definition.component
                    ),
                })
            })?;

        let wayland_config = WaylandSurfaceConfig::from_domain_config(
            &definition.component,
            component_definition,
            Some(Rc::clone(&self.compilation_result)),
            definition.config.clone(),
        );

        let shell_config = ShellSurfaceConfig {
            name: definition.component.clone(),
            config: wayland_config,
        };

        let mut facade = self.inner.borrow_mut();
        let handles = facade.inner_mut().spawn_surface(&shell_config)?;

        let surface_handle = SurfaceHandle::new();
        self.surface_handles
            .insert(surface_handle, definition.component.clone());
        self.surfaces
            .insert(definition.component.clone(), definition);

        log::info!(
            "Spawned surface with handle {:?}, created {} output instances",
            surface_handle,
            handles.len()
        );

        Ok(vec![surface_handle])
    }

    pub fn despawn_surface(&mut self, handle: SurfaceHandle) -> Result<()> {
        let surface_name = self.surface_handles.remove(&handle).ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: format!("Surface handle {:?} not found", handle),
            })
        })?;

        self.surfaces.remove(&surface_name);

        let mut facade = self.inner.borrow_mut();
        facade.inner_mut().despawn_surface(&surface_name)?;

        log::info!(
            "Despawned surface '{}' with handle {:?}",
            surface_name,
            handle
        );

        Ok(())
    }

    pub fn on_output_connected<F>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(&OutputInfo) + 'static,
    {
        self.output_connected_handlers
            .borrow_mut()
            .push(Box::new(handler));
        Ok(())
    }

    pub fn on_output_disconnected<F>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(OutputHandle) + 'static,
    {
        self.output_disconnected_handlers
            .borrow_mut()
            .push(Box::new(handler));
        Ok(())
    }

    pub fn get_surface_handle(&self, name: &str) -> Option<SurfaceHandle> {
        self.surface_handles
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(h, _)| *h)
    }

    pub fn get_surface_name(&self, handle: SurfaceHandle) -> Option<&str> {
        self.surface_handles.get(&handle).map(String::as_str)
    }

    pub fn with_surface<F, R>(&self, name: &str, f: F) -> Result<R>
    where
        F: FnOnce(&ComponentInstance) -> R,
    {
        if !self.surfaces.contains_key(name) {
            return Err(Error::Domain(DomainError::Configuration {
                message: format!("Window '{}' not found", name),
            }));
        }

        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        system
            .app_state()
            .surfaces_by_name(name)
            .next()
            .map(|surface| f(surface.component_instance()))
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: format!("No instance found for window '{}'", name),
                })
            })
    }

    pub fn with_all_surfaces<F>(&self, mut f: F)
    where
        F: FnMut(&str, &ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for name in self.surfaces.keys() {
            for surface in system.app_state().surfaces_by_name(name) {
                f(name, surface.component_instance());
            }
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
        for (handle, surface) in system.app_state().outputs_with_handles() {
            f(handle, surface.component_instance());
        }
    }

    #[must_use]
    pub fn compilation_result(&self) -> &Rc<CompilationResult> {
        &self.compilation_result
    }

    #[must_use]
    pub fn popup(&self, component_name: impl Into<String>) -> PopupBuilder<'_> {
        PopupBuilder::new(self, component_name.into())
    }

    pub fn on<F, R>(&self, surface_name: &str, callback_name: &str, handler: F) -> Result<()>
    where
        F: Fn(ShellControl) -> R + 'static,
        R: IntoValue,
    {
        if !self.surfaces.contains_key(surface_name) {
            return Err(Error::Domain(DomainError::Configuration {
                message: format!("Window '{}' not found", surface_name),
            }));
        }

        let control = self.control();
        let handler = Rc::new(handler);
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for surface in system.app_state().surfaces_by_name(surface_name) {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = surface
                .component_instance()
                .set_callback(callback_name, move |_args| {
                    handler_rc(control_clone.clone()).into_value()
                })
            {
                log::error!(
                    "Failed to register callback '{}' on surface '{}': {}",
                    callback_name,
                    surface_name,
                    e
                );
            }
        }

        Ok(())
    }

    pub fn on_with_args<F, R>(
        &self,
        surface_name: &str,
        callback_name: &str,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&[Value], ShellControl) -> R + 'static,
        R: IntoValue,
    {
        if !self.surfaces.contains_key(surface_name) {
            return Err(Error::Domain(DomainError::Configuration {
                message: format!("Window '{}' not found", surface_name),
            }));
        }

        let control = self.control();
        let handler = Rc::new(handler);
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for surface in system.app_state().surfaces_by_name(surface_name) {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = surface
                .component_instance()
                .set_callback(callback_name, move |args| {
                    handler_rc(args, control_clone.clone()).into_value()
                })
            {
                log::error!(
                    "Failed to register callback '{}' on surface '{}': {}",
                    callback_name,
                    surface_name,
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

        for surface in system.app_state().all_outputs() {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = surface
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

        for surface in system.app_state().all_outputs() {
            let handler_rc = Rc::clone(&handler);
            let control_clone = control.clone();
            if let Err(e) = surface
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

    pub fn apply_surface_config<F>(&self, surface_name: &str, f: F)
    where
        F: Fn(&ComponentInstance, LayerSurfaceHandle<'_>),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        if self.surfaces.contains_key(surface_name) {
            for surface in system.app_state().surfaces_by_name(surface_name) {
                let surface_handle = LayerSurfaceHandle::from_window_state(surface);
                f(surface.component_instance(), surface_handle);
            }
        }
    }

    pub fn apply_global_config<F>(&self, f: F)
    where
        F: Fn(&ComponentInstance, LayerSurfaceHandle<'_>),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for surface in system.app_state().all_outputs() {
            let surface_handle = LayerSurfaceHandle::from_window_state(surface);
            f(surface.component_instance(), surface_handle);
        }
    }

    pub fn output_registry(&self) -> OutputRegistry {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();
        system.app_state().output_registry().clone()
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

        if self.surfaces.contains_key(name) {
            for surface in system.app_state().surfaces_by_name(name) {
                f(surface.component_instance());
            }
        }
    }

    fn with_all_components<F>(&self, mut f: F)
    where
        F: FnMut(&str, &ComponentInstance),
    {
        let facade = self.inner.borrow();
        let system = facade.inner_ref();

        for name in self.surfaces.keys() {
            for surface in system.app_state().surfaces_by_name(name) {
                f(name, surface.component_instance());
            }
        }
    }

    fn run(&mut self) -> Result<()> {
        self.inner.borrow_mut().run()?;
        Ok(())
    }
}

pub type ShellEventLoopHandle = EventLoopHandleBase;

pub struct ShellEventContext<'a> {
    app_state: &'a mut AppState,
}

impl<'a> FromAppState<'a> for ShellEventContext<'a> {
    fn from_app_state(app_state: &'a mut AppState) -> Self {
        Self { app_state }
    }
}

impl ShellEventContext<'_> {
    pub fn get_surface_component(&self, name: &str) -> Option<&ComponentInstance> {
        self.app_state
            .surfaces_by_name(name)
            .next()
            .map(SurfaceState::component_instance)
    }

    pub fn all_surface_components(&self) -> impl Iterator<Item = &ComponentInstance> {
        self.app_state
            .all_outputs()
            .map(SurfaceState::component_instance)
    }

    pub fn render_frame_if_dirty(&mut self) -> Result<()> {
        for surface in self.app_state.all_outputs() {
            surface.render_frame_if_dirty()?;
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
            .map(|(handle, surface)| (handle, surface.component_instance()))
    }

    pub fn get_output_component(&self, handle: OutputHandle) -> Option<&ComponentInstance> {
        self.app_state
            .get_output_by_handle(handle)
            .map(SurfaceState::component_instance)
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
            .map(|(info, surface)| (info, surface.component_instance()))
    }

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.app_state
            .primary_output()
            .and_then(SurfaceState::compilation_result)
    }
}
