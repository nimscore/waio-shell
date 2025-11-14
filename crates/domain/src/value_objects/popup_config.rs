use super::popup_positioning_mode::PopupPositioningMode;
use crate::dimensions::{LogicalPosition, LogicalSize};
use crate::surface_dimensions::SurfaceDimensions;

#[derive(Debug, Clone, Copy)]
pub struct PopupConfig {
    reference_position: LogicalPosition,
    dimensions: SurfaceDimensions,
    output_bounds: LogicalSize,
    positioning_mode: PopupPositioningMode,
}

impl PopupConfig {
    pub fn new(
        reference_x: f32,
        reference_y: f32,
        dimensions: SurfaceDimensions,
        positioning_mode: PopupPositioningMode,
        output_bounds: LogicalSize,
    ) -> Self {
        Self {
            reference_position: LogicalPosition::new(reference_x, reference_y),
            dimensions,
            output_bounds,
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

    pub const fn dimensions(&self) -> SurfaceDimensions {
        self.dimensions
    }

    pub fn popup_size(&self) -> LogicalSize {
        self.dimensions.logical_size()
    }

    pub fn width(&self) -> f32 {
        self.dimensions.logical_size().width()
    }

    pub fn height(&self) -> f32 {
        self.dimensions.logical_size().height()
    }

    pub const fn output_bounds(&self) -> LogicalSize {
        self.output_bounds
    }

    pub const fn positioning_mode(&self) -> PopupPositioningMode {
        self.positioning_mode
    }

    pub fn calculated_top_left_position(&self) -> LogicalPosition {
        let unclamped = self.calculate_unclamped_position();
        self.popup_size()
            .clamp_position(unclamped, self.output_bounds)
    }

    fn calculate_unclamped_position(&self) -> LogicalPosition {
        let x = if self.positioning_mode.center_x() {
            self.reference_x() - (self.width() / 2.0)
        } else {
            self.reference_x()
        };

        let y = if self.positioning_mode.center_y() {
            self.reference_y() - (self.height() / 2.0)
        } else {
            self.reference_y()
        };

        LogicalPosition::new(x, y)
    }

    pub fn calculated_top_left_x(&self) -> f32 {
        self.calculated_top_left_position().x()
    }

    pub fn calculated_top_left_y(&self) -> f32 {
        self.calculated_top_left_position().y()
    }
}
