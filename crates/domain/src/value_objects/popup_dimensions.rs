use crate::dimensions::LogicalSize;
use crate::errors::{DomainError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PopupDimensions {
    size: LogicalSize,
}

impl PopupDimensions {
    pub fn new(width: f32, height: f32) -> Result<Self> {
        let size = LogicalSize::new(width, height)?;
        Ok(Self { size })
    }

    pub const fn from_logical(size: LogicalSize) -> Self {
        Self { size }
    }

    pub const fn width(&self) -> f32 {
        self.size.width()
    }

    pub const fn height(&self) -> f32 {
        self.size.height()
    }

    pub const fn logical_size(&self) -> LogicalSize {
        self.size
    }

    pub fn as_tuple(&self) -> (f32, f32) {
        self.size.as_tuple()
    }
}

impl TryFrom<(f32, f32)> for PopupDimensions {
    type Error = DomainError;

    fn try_from((width, height): (f32, f32)) -> Result<Self> {
        Self::new(width, height)
    }
}
