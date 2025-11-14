#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

impl Default for Layer {
    fn default() -> Self {
        Self::Top
    }
}
