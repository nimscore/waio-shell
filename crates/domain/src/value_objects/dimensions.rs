use crate::errors::{DomainError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowHeight(u32);

impl WindowHeight {
    pub fn new(height: u32) -> Result<Self> {
        if height == 0 {
            return Err(DomainError::InvalidDimensions {
                width: 0,
                height: 0,
            });
        }
        Ok(Self(height))
    }

    pub const fn from_raw(height: u32) -> Self {
        Self(height)
    }

    pub const fn value(&self) -> u32 {
        self.0
    }
}

impl Default for WindowHeight {
    fn default() -> Self {
        Self(30)
    }
}

impl TryFrom<u32> for WindowHeight {
    type Error = DomainError;

    fn try_from(height: u32) -> Result<Self> {
        Self::new(height)
    }
}
