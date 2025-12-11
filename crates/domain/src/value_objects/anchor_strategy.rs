/// Strategy for calculating popup position relative to an anchor rectangle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorStrategy {
    /// Center popup horizontally below the anchor
    CenterBottom,
    /// Center popup horizontally above the anchor
    CenterTop,
    /// Position popup at anchor's bottom-left
    LeftBottom,
    /// Position popup at anchor's bottom-right
    RightBottom,
    /// Position popup at anchor's top-left
    LeftTop,
    /// Position popup at anchor's top-right
    RightTop,
    /// Position popup at cursor coordinates
    Cursor,
}

impl AnchorStrategy {
    #[must_use]
    pub const fn calculate_position(
        self,
        anchor_x: f64,
        anchor_y: f64,
        anchor_w: f64,
        anchor_h: f64,
        popup_w: f64,
        popup_h: f64,
    ) -> (f64, f64) {
        match self {
            Self::CenterBottom => {
                let center_x = anchor_x + (anchor_w / 2.0);
                let x = center_x - (popup_w / 2.0);
                let y = anchor_y + anchor_h;
                (x, y)
            }
            Self::CenterTop => {
                let center_x = anchor_x + (anchor_w / 2.0);
                let x = center_x - (popup_w / 2.0);
                let y = anchor_y - popup_h;
                (x, y)
            }
            Self::LeftBottom => (anchor_x, anchor_y + anchor_h),
            Self::RightBottom => (anchor_x + anchor_w - popup_w, anchor_y + anchor_h),
            Self::LeftTop => (anchor_x, anchor_y - popup_h),
            Self::RightTop => (anchor_x + anchor_w - popup_w, anchor_y - popup_h),
            Self::Cursor => (anchor_x, anchor_y),
        }
    }
}

impl Default for AnchorStrategy {
    fn default() -> Self {
        Self::CenterBottom
    }
}
