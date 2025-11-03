use crate::errors::{DomainError, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupDimensions {
    width: f32,
    height: f32,
}

impl PopupDimensions {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(&self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> f32 {
        self.height
    }

    pub fn validate(&self) -> Result<()> {
        if self.width <= 0.0 || self.height <= 0.0 {
            return Err(DomainError::Configuration {
                message: format!(
                    "Invalid popup dimensions: width={}, height={}. Both must be positive.",
                    self.width, self.height
                ),
            });
        }
        Ok(())
    }
}

impl Default for PopupDimensions {
    fn default() -> Self {
        Self::new(120.0, 120.0)
    }
}
