use super::popup_positioning_mode::PopupPositioningMode;

#[derive(Debug, Clone, Copy)]
pub struct PopupConfig {
    reference_x: f32,
    reference_y: f32,
    width: f32,
    height: f32,
    positioning_mode: PopupPositioningMode,
    output_width: f32,
    output_height: f32,
}

impl PopupConfig {
    #[must_use]
    pub const fn new(
        reference_x: f32,
        reference_y: f32,
        width: f32,
        height: f32,
        positioning_mode: PopupPositioningMode,
        output_width: f32,
        output_height: f32,
    ) -> Self {
        Self {
            reference_x,
            reference_y,
            width,
            height,
            positioning_mode,
            output_width,
            output_height,
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
        let unclamped_x = if self.positioning_mode.center_x() {
            self.reference_x - (self.width / 2.0)
        } else {
            self.reference_x
        };

        let max_x = self.output_width - self.width;
        unclamped_x.max(0.0).min(max_x)
    }

    #[must_use]
    pub fn calculated_top_left_y(&self) -> f32 {
        let unclamped_y = if self.positioning_mode.center_y() {
            self.reference_y - (self.height / 2.0)
        } else {
            self.reference_y
        };

        let max_y = self.output_height - self.height;
        unclamped_y.max(0.0).min(max_y)
    }
}
