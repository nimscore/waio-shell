use crate::Result;
use crate::system::SingleWindowShell;
use layer_shika_adapters::platform::slint_interpreter::{CompilationResult, Compiler};
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::prelude::{
    AnchorEdges, KeyboardInteractivity, Layer, Margins, OutputPolicy, ScaleFactor, WindowConfig,
    WindowDimension,
};
use spin_on::spin_on;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct NeedsComponent;
pub struct HasComponent {
    component_name: String,
    compilation_result: Rc<CompilationResult>,
}

pub struct LayerShika<State> {
    state: State,
    config: WindowConfig,
}

impl LayerShika<NeedsComponent> {
    #[must_use]
    pub fn new(
        compilation_result: Rc<CompilationResult>,
        component_name: impl Into<String>,
    ) -> LayerShika<HasComponent> {
        LayerShika {
            state: HasComponent {
                component_name: component_name.into(),
                compilation_result,
            },
            config: WindowConfig::default(),
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<LayerShika<HasComponent>> {
        Self::from_file_with_component(path, "Main")
    }

    pub fn from_file_with_component(
        path: impl AsRef<Path>,
        component_name: impl AsRef<str>,
    ) -> Result<LayerShika<HasComponent>> {
        Self::from_file_with_compiler(path, &mut Compiler::default(), component_name.as_ref())
    }

    pub fn from_file_with_compiler(
        path: impl AsRef<Path>,
        compiler: &mut Compiler,
        component_name: &str,
    ) -> Result<LayerShika<HasComponent>> {
        let compilation_result = spin_on(compiler.build_from_path(path.as_ref()));
        let diagnostics: Vec<_> = compilation_result.diagnostics().collect();
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

        compilation_result
            .component(component_name)
            .ok_or_else(|| DomainError::Configuration {
                message: format!(
                    "Component '{}' not found in Slint file '{}'",
                    component_name,
                    path.as_ref().display()
                ),
            })?;

        Ok(LayerShika {
            state: HasComponent {
                component_name: component_name.to_string(),
                compilation_result: Rc::new(compilation_result),
            },
            config: WindowConfig::default(),
        })
    }

    pub fn from_source(source: impl AsRef<str>) -> Result<LayerShika<HasComponent>> {
        Self::from_source_with_component(source, "Main")
    }

    pub fn from_source_with_component(
        source: impl AsRef<str>,
        component_name: impl AsRef<str>,
    ) -> Result<LayerShika<HasComponent>> {
        Self::from_source_with_compiler(source, &mut Compiler::default(), component_name.as_ref())
    }

    pub fn from_source_with_compiler(
        source: impl AsRef<str>,
        compiler: &mut Compiler,
        component_name: &str,
    ) -> Result<LayerShika<HasComponent>> {
        let compilation_result =
            spin_on(compiler.build_from_source(source.as_ref().to_string(), PathBuf::default()));

        let diagnostics: Vec<_> = compilation_result.diagnostics().collect();
        if !diagnostics.is_empty() {
            let messages: Vec<String> = diagnostics.iter().map(ToString::to_string).collect();
            return Err(DomainError::Configuration {
                message: format!(
                    "Failed to compile Slint source code:\n{}",
                    messages.join("\n")
                ),
            }
            .into());
        }

        compilation_result
            .component(component_name)
            .ok_or_else(|| DomainError::Configuration {
                message: format!(
                    "Component '{}' not found in Slint source code",
                    component_name
                ),
            })?;

        Ok(LayerShika {
            state: HasComponent {
                component_name: component_name.to_string(),
                compilation_result: Rc::new(compilation_result),
            },
            config: WindowConfig::default(),
        })
    }
}

impl LayerShika<HasComponent> {
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.config.dimensions = WindowDimension::new(width, height);
        self
    }

    #[must_use]
    pub fn height(mut self, height: u32) -> Self {
        self.config.dimensions = WindowDimension::new(self.config.dimensions.width(), height);
        self
    }

    #[must_use]
    pub fn width(mut self, width: u32) -> Self {
        self.config.dimensions = WindowDimension::new(width, self.config.dimensions.height());
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

    pub fn build(self) -> Result<SingleWindowShell> {
        let component_definition = self
            .state
            .compilation_result
            .component(&self.state.component_name)
            .ok_or_else(|| DomainError::Configuration {
                message: format!(
                    "Component '{}' not found in compilation result",
                    self.state.component_name
                ),
            })?;

        SingleWindowShell::new(
            component_definition,
            Some(self.state.compilation_result),
            self.config,
        )
    }

    pub fn run(self) -> Result<()> {
        let mut app = self.build()?;
        app.run()
    }
}
