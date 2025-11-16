use crate::Result;
use crate::system::WindowingSystem;
use layer_shika_adapters::platform::slint_interpreter::{CompilationResult, Compiler};
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::prelude::{
    AnchorEdges, KeyboardInteractivity, Layer, Margins, ScaleFactor, WindowConfig, WindowHeight,
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

    pub fn from_file(
        path: impl AsRef<Path>,
        component_name: Option<&str>,
    ) -> Result<LayerShika<HasComponent>> {
        Self::from_file_with_compiler(
            path,
            &mut Compiler::default(),
            component_name.unwrap_or("Main"),
        )
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

    pub fn from_source(
        source: impl AsRef<str>,
        component_name: Option<&str>,
    ) -> Result<LayerShika<HasComponent>> {
        Self::from_source_with_compiler(
            source,
            &mut Compiler::default(),
            component_name.unwrap_or("Main"),
        )
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
    pub fn with_height(mut self, height: u32) -> Self {
        self.config.height = WindowHeight::new(height);
        self
    }

    #[must_use]
    pub const fn with_layer(mut self, layer: Layer) -> Self {
        self.config.layer = layer;
        self
    }

    #[must_use]
    pub const fn with_margin(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.config.margin = Margins {
            top,
            right,
            bottom,
            left,
        };
        self
    }

    #[must_use]
    pub const fn with_anchor(mut self, anchor: AnchorEdges) -> Self {
        self.config.anchor = anchor;
        self
    }

    #[must_use]
    pub const fn with_exclusive_zone(mut self, zone: i32) -> Self {
        self.config.exclusive_zone = zone;
        self
    }

    #[must_use]
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.config.namespace = namespace;
        self
    }

    pub fn with_scale_factor(mut self, scale_factor: f32) -> Result<Self> {
        self.config.scale_factor = ScaleFactor::new(scale_factor)?;
        Ok(self)
    }

    #[must_use]
    pub const fn with_keyboard_interactivity(mut self, mode: KeyboardInteractivity) -> Self {
        self.config.keyboard_interactivity = mode;
        self
    }

    pub fn build(self) -> Result<WindowingSystem> {
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

        WindowingSystem::new(
            component_definition,
            Some(self.state.compilation_result),
            self.config,
        )
    }
}
