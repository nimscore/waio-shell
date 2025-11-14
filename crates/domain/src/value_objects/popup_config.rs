use super::popup_positioning_mode::PopupPositioningMode;
use crate::dimensions::{LogicalPosition, LogicalSize};

#[derive(Debug, Clone, Copy)]
pub struct PopupConfig {
    reference_position: LogicalPosition,
    popup_size: LogicalSize,
    output_size: LogicalSize,
    positioning_mode: PopupPositioningMode,
}

impl PopupConfig {
    pub fn new(
        reference_x: f32,
        reference_y: f32,
        width: f32,
        height: f32,
        positioning_mode: PopupPositioningMode,
        output_width: f32,
        output_height: f32,
    ) -> Self {
        Self {
            reference_position: LogicalPosition::new(reference_x, reference_y),
            popup_size: LogicalSize::from_raw(width, height),
            output_size: LogicalSize::from_raw(output_width, output_height),
            positioning_mode,
        }
    }

    pub const fn reference_position(&self) -> LogicalPosition {
        self.reference_position
    }

    pub const fn reference_x(&self) -> f32 {
        self.reference_position.x()
    }

    pub const fn reference_y(&self) -> f32 {
        self.reference_position.y()
    }

    pub const fn popup_size(&self) -> LogicalSize {
        self.popup_size
    }

    pub const fn width(&self) -> f32 {
        self.popup_size.width()
    }

    pub const fn height(&self) -> f32 {
        self.popup_size.height()
    }

    pub const fn output_size(&self) -> LogicalSize {
        self.output_size
    }

    pub const fn positioning_mode(&self) -> PopupPositioningMode {
        self.positioning_mode
    }

    pub fn calculated_top_left_position(&self) -> LogicalPosition {
        LogicalPosition::new(self.calculated_top_left_x(), self.calculated_top_left_y())
    }

    pub fn calculated_top_left_x(&self) -> f32 {
        let unclamped_x = if self.positioning_mode.center_x() {
            self.reference_x() - (self.width() / 2.0)
        } else {
            self.reference_x()
        };

        let max_x = self.output_size.width() - self.width();
        unclamped_x.max(0.0).min(max_x)
    }

    pub fn calculated_top_left_y(&self) -> f32 {
        let unclamped_y = if self.positioning_mode.center_y() {
            self.reference_y() - (self.height() / 2.0)
        } else {
            self.reference_y()
        };

        let max_y = self.output_size.height() - self.height();
        unclamped_y.max(0.0).min(max_y)
    }
}
