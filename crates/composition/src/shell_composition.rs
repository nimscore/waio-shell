use crate::shell::Shell;
use crate::{Error, Result};
use layer_shika_adapters::platform::slint_interpreter::CompilationResult;
use layer_shika_domain::config::WindowConfig;
use layer_shika_domain::errors::DomainError;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ShellWindowDefinition {
    pub component_name: String,
    pub config: WindowConfig,
}

#[must_use]
pub struct ShellComposition {
    compilation_result: Option<Rc<CompilationResult>>,
    shell_windows: Vec<ShellWindowDefinition>,
    auto_discover_components: Vec<String>,
}

impl ShellComposition {
    pub fn new() -> Self {
        Self {
            compilation_result: None,
            shell_windows: Vec::new(),
            auto_discover_components: Vec::new(),
        }
    }

    pub fn with_compilation_result(mut self, result: Rc<CompilationResult>) -> Self {
        self.compilation_result = Some(result);
        self
    }

    pub fn register_shell_window(
        mut self,
        component_name: impl Into<String>,
        config: WindowConfig,
    ) -> Self {
        self.shell_windows.push(ShellWindowDefinition {
            component_name: component_name.into(),
            config,
        });
        self
    }

    pub fn register_shell_windows(mut self, definitions: Vec<ShellWindowDefinition>) -> Self {
        self.shell_windows.extend(definitions);
        self
    }

    pub fn auto_discover(mut self, component_names: Vec<impl Into<String>>) -> Self {
        self.auto_discover_components = component_names.into_iter().map(Into::into).collect();
        self
    }

    pub fn build(self) -> Result<Shell> {
        let compilation_result = self.compilation_result.ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No compilation result provided. Use with_compilation_result()"
                    .to_string(),
            })
        })?;

        if !self.auto_discover_components.is_empty() {
            return Shell::new_auto_discover(compilation_result, &self.auto_discover_components);
        }

        if self.shell_windows.is_empty() {
            return Err(Error::Domain(DomainError::Configuration {
                message: "No shell windows registered. Use register_shell_window(), register_shell_windows(), or auto_discover()".to_string(),
            }));
        }

        Shell::new(compilation_result, &self.shell_windows)
    }
}

impl Default for ShellComposition {
    fn default() -> Self {
        Self::new()
    }
}
