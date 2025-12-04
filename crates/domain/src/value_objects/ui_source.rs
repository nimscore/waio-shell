use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum UiSource {
    File(PathBuf),
    Source(String),
}

impl UiSource {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File(path.into())
    }

    pub fn source(code: impl Into<String>) -> Self {
        Self::Source(code.into())
    }
}

impl From<&str> for UiSource {
    fn from(s: &str) -> Self {
        Self::File(PathBuf::from(s))
    }
}

impl From<String> for UiSource {
    fn from(s: String) -> Self {
        Self::File(PathBuf::from(s))
    }
}

impl From<PathBuf> for UiSource {
    fn from(path: PathBuf) -> Self {
        Self::File(path)
    }
}
