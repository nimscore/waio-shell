use crate::dimensions::LogicalRect;

#[derive(Debug, Clone)]
pub enum PopupPosition {
    /// Absolute position in surface coordinates
    Absolute { x: f32, y: f32 },

    /// Relative to cursor position
    Cursor { offset: Offset },

    /// Relative to a UI element (rect)
    Element {
        rect: LogicalRect,
        anchor: AnchorPoint,
        alignment: Alignment,
    },

    /// Relative to parent popup
    RelativeToParent {
        anchor: AnchorPoint,
        alignment: Alignment,
        offset: Offset,
    },

    /// Centered on output
    Centered { offset: Offset },
}

/// 9-point anchor system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorPoint {
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

/// How popup aligns relative to anchor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Popup starts at anchor
    Start,
    /// Popup centers on anchor
    Center,
    /// Popup ends at anchor
    End,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}
