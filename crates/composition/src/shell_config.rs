use layer_shika_adapters::platform::slint_interpreter::CompilationResult;
use layer_shika_domain::prelude::{SurfaceConfig, UiSource};
use std::path::PathBuf;
use std::rc::Rc;

pub enum CompiledUiSource {
    File(PathBuf),
    Source(String),
    Compiled(Rc<CompilationResult>),
}

impl CompiledUiSource {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    pub fn source(code: impl Into<String>) -> Self {
        Self::Source(code.into())
    }

    pub fn compiled(result: Rc<CompilationResult>) -> Self {
        Self::Compiled(result)
    }
}

impl From<UiSource> for CompiledUiSource {
    fn from(source: UiSource) -> Self {
        match source {
            UiSource::File(path) => Self::File(path),
            UiSource::Source(code) => Self::Source(code),
        }
    }
}

impl From<Rc<CompilationResult>> for CompiledUiSource {
    fn from(result: Rc<CompilationResult>) -> Self {
        Self::Compiled(result)
    }
}

impl From<&str> for CompiledUiSource {
    fn from(s: &str) -> Self {
        Self::File(PathBuf::from(s))
    }
}

impl From<String> for CompiledUiSource {
    fn from(s: String) -> Self {
        Self::File(PathBuf::from(s))
    }
}

impl From<PathBuf> for CompiledUiSource {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}

pub struct ShellConfig {
    pub ui_source: CompiledUiSource,
    pub surfaces: Vec<SurfaceComponentConfig>,
}

#[derive(Debug, Clone)]
pub struct SurfaceComponentConfig {
    pub component: String,
    pub config: SurfaceConfig,
}

impl ShellConfig {
    pub fn new(ui_source: impl Into<CompiledUiSource>) -> Self {
        Self {
            ui_source: ui_source.into(),
            surfaces: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_surface(mut self, component: impl Into<String>) -> Self {
        self.surfaces.push(SurfaceComponentConfig {
            component: component.into(),
            config: SurfaceConfig::default(),
        });
        self
    }

    #[must_use]
    pub fn with_surface_config(
        mut self,
        component: impl Into<String>,
        config: SurfaceConfig,
    ) -> Self {
        self.surfaces.push(SurfaceComponentConfig {
            component: component.into(),
            config,
        });
        self
    }

    pub fn add_surface(&mut self, component: impl Into<String>) -> &mut SurfaceComponentConfig {
        self.surfaces.push(SurfaceComponentConfig {
            component: component.into(),
            config: SurfaceConfig::default(),
        });
        self.surfaces
            .last_mut()
            .unwrap_or_else(|| unreachable!("just pushed"))
    }

    pub fn add_surface_config(
        &mut self,
        component: impl Into<String>,
        config: SurfaceConfig,
    ) -> &mut SurfaceComponentConfig {
        self.surfaces.push(SurfaceComponentConfig {
            component: component.into(),
            config,
        });
        self.surfaces
            .last_mut()
            .unwrap_or_else(|| unreachable!("just pushed"))
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            ui_source: CompiledUiSource::Source(String::new()),
            surfaces: Vec::new(),
        }
    }
}

impl SurfaceComponentConfig {
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            config: SurfaceConfig::default(),
        }
    }

    pub fn with_config(component: impl Into<String>, config: SurfaceConfig) -> Self {
        Self {
            component: component.into(),
            config,
        }
    }
}
