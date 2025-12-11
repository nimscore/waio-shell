use crate::{OutputHandle, OutputInfo};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SurfaceInfo {
    pub name: String,
    pub output: OutputHandle,
}

#[derive(Clone)]
pub enum Surface {
    All,
    Named(String),
    Any(Vec<String>),
    Filter(Arc<dyn Fn(&SurfaceInfo) -> bool + Send + Sync>),
}

impl Surface {
    pub fn all() -> Self {
        Self::All
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    pub fn any(names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Any(names.into_iter().map(Into::into).collect())
    }

    pub fn matching<F>(predicate: F) -> Self
    where
        F: Fn(&SurfaceInfo) -> bool + Send + Sync + 'static,
    {
        Self::Filter(Arc::new(predicate))
    }

    pub fn on(self, output: Output) -> Selector {
        Selector {
            surface: self,
            output,
        }
    }

    pub(crate) fn matches(&self, info: &SurfaceInfo) -> bool {
        match self {
            Self::All => true,
            Self::Named(name) => &info.name == name,
            Self::Any(names) => names.iter().any(|name| name == &info.name),
            Self::Filter(predicate) => predicate(info),
        }
    }
}

impl Debug for Surface {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::All => write!(f, "Surface::All"),
            Self::Named(name) => write!(f, "Surface::Named({:?})", name),
            Self::Any(names) => write!(f, "Surface::Any({:?})", names),
            Self::Filter(_) => write!(f, "Surface::Filter(<fn>)"),
        }
    }
}

#[derive(Clone)]
pub enum Output {
    All,
    Primary,
    Active,
    Handle(OutputHandle),
    Named(String),
    Filter(Arc<dyn Fn(&OutputInfo) -> bool + Send + Sync>),
}

impl Output {
    pub fn all() -> Self {
        Self::All
    }

    pub fn primary() -> Self {
        Self::Primary
    }

    pub fn active() -> Self {
        Self::Active
    }

    pub fn handle(handle: OutputHandle) -> Self {
        Self::Handle(handle)
    }

    pub fn named(name: impl Into<String>) -> Self {
        Self::Named(name.into())
    }

    pub fn matching<F>(predicate: F) -> Self
    where
        F: Fn(&OutputInfo) -> bool + Send + Sync + 'static,
    {
        Self::Filter(Arc::new(predicate))
    }

    pub(crate) fn matches(
        &self,
        handle: OutputHandle,
        info: Option<&OutputInfo>,
        primary: Option<OutputHandle>,
        active: Option<OutputHandle>,
    ) -> bool {
        match self {
            Self::All => true,
            Self::Primary => primary == Some(handle),
            Self::Active => active == Some(handle),
            Self::Handle(h) => *h == handle,
            Self::Named(name) => info.is_some_and(|i| i.name() == Some(name.as_str())),
            Self::Filter(predicate) => info.is_some_and(|i| predicate(i)),
        }
    }
}

impl Debug for Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::All => write!(f, "Output::All"),
            Self::Primary => write!(f, "Output::Primary"),
            Self::Active => write!(f, "Output::Active"),
            Self::Handle(h) => write!(f, "Output::Handle({:?})", h),
            Self::Named(name) => write!(f, "Output::Named({:?})", name),
            Self::Filter(_) => write!(f, "Output::Filter(<fn>)"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Selector {
    pub surface: Surface,
    pub output: Output,
}

impl Selector {
    pub fn all() -> Self {
        Self {
            surface: Surface::All,
            output: Output::All,
        }
    }

    pub(crate) fn matches(
        &self,
        surface_info: &SurfaceInfo,
        output_info: Option<&OutputInfo>,
        primary: Option<OutputHandle>,
        active: Option<OutputHandle>,
    ) -> bool {
        self.surface.matches(surface_info)
            && self
                .output
                .matches(surface_info.output, output_info, primary, active)
    }
}

impl From<Surface> for Selector {
    fn from(surface: Surface) -> Self {
        Self {
            surface,
            output: Output::All,
        }
    }
}

impl From<Output> for Selector {
    fn from(output: Output) -> Self {
        Self {
            surface: Surface::All,
            output,
        }
    }
}
