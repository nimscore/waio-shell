#[derive(Debug, Clone, Copy)]
pub struct WindowHeight(u32);

impl WindowHeight {
    #[must_use]
    pub const fn new(height: u32) -> Self {
        Self(height)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

impl Default for WindowHeight {
    fn default() -> Self {
        Self(30)
    }
}
