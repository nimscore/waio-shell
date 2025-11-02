use super::popup_positioning_mode::PopupPositioningMode;

#[derive(Debug, Clone, Copy)]
pub struct PopupConfig {
    reference_x: f32,
    reference_y: f32,
    width: f32,
    height: f32,
    positioning_mode: PopupPositioningMode,
}

impl PopupConfig {
    #[must_use]
    pub const fn new(
        reference_x: f32,
        reference_y: f32,
        width: f32,
        height: f32,
        positioning_mode: PopupPositioningMode,
    ) -> Self {
        Self {
            reference_x,
            reference_y,
            width,
            height,
            positioning_mode,
        }
    }

    #[must_use]
    pub const fn reference_x(&self) -> f32 {
        self.reference_x
    }

    #[must_use]
    pub const fn reference_y(&self) -> f32 {
        self.reference_y
    }

    #[must_use]
    pub const fn width(&self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> f32 {
        self.height
    }

    #[must_use]
    pub const fn positioning_mode(&self) -> PopupPositioningMode {
        self.positioning_mode
    }

    #[must_use]
    pub fn calculated_top_left_x(&self) -> f32 {
        if self.positioning_mode.center_x() {
            self.reference_x - (self.width / 2.0)
        } else {
            self.reference_x
        }
    }

    #[must_use]
    pub fn calculated_top_left_y(&self) -> f32 {
        if self.positioning_mode.center_y() {
            self.reference_y - (self.height / 2.0)
        } else {
            self.reference_y
        }
    }
}
