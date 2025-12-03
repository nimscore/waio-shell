#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowDimension {
    width: u32,
    height: u32,
}

impl WindowDimension {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: if width == 0 {
                Self::default().width
            } else {
                width
            },
            height: if height == 0 {
                Self::default().height
            } else {
                height
            },
        }
    }

    pub const fn from_raw(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }
}

impl Default for WindowDimension {
    fn default() -> Self {
        Self {
            width: 20,
            height: 20,
        }
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
