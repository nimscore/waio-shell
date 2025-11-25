
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupDimensions {
    pub width: f32,
    pub height: f32,
}

impl Default for PopupDimensions {
    fn default() -> Self {
        Self {
            width: 200.0,
            height: 150.0,
        }
    }
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
}
