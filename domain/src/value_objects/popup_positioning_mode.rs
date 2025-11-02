#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PopupPositioningMode {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl PopupPositioningMode {
    #[must_use]
    pub const fn center_x(self) -> bool {
        matches!(self, Self::TopCenter | Self::Center | Self::BottomCenter)
    }

    #[must_use]
    pub const fn center_y(self) -> bool {
        matches!(self, Self::CenterLeft | Self::Center | Self::CenterRight)
    }

    #[must_use]
    pub const fn from_flags(center_x: bool, center_y: bool) -> Self {
        match (center_x, center_y) {
            (false, false) => Self::TopLeft,
            (true, false) => Self::TopCenter,
            (false, true) => Self::CenterLeft,
            (true, true) => Self::Center,
        }
    }
}

impl Default for PopupPositioningMode {
    fn default() -> Self {
        Self::TopLeft
    }
}
