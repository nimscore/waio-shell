/// Alignment mode for popup positioning
///
/// Determines how a popup is aligned relative to its placement point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PopupPositioningMode {
    /// Align popup's top-left corner to placement point
    TopLeft,
    /// Center popup horizontally at placement point, top edge aligned
    TopCenter,
    /// Align popup's top-right corner to placement point
    TopRight,
    /// Center popup vertically at placement point, left edge aligned
    CenterLeft,
    /// Center popup both horizontally and vertically at placement point
    Center,
    /// Center popup vertically at placement point, right edge aligned
    CenterRight,
    /// Align popup's bottom-left corner to placement point
    BottomLeft,
    /// Center popup horizontally at placement point, bottom edge aligned
    BottomCenter,
    /// Align popup's bottom-right corner to placement point
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
