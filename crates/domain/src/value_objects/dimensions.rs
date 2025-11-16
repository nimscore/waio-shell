
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowHeight(u32);

impl WindowHeight {
    pub fn new(height: u32) -> Self {
        if height == 0 {
            Self::default()
        } else {
            Self(height)
        }
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

impl From<u32> for WindowHeight {
    fn from(height: u32) -> Self {
        Self::new(height)
    }
}
