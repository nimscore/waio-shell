#[derive(Debug, Clone, Copy)]
pub struct SurfaceDimensions {
    pub logical_width: u32,
    pub logical_height: u32,
    pub physical_width: u32,
    pub physical_height: u32,
    pub buffer_scale: i32,
}

impl SurfaceDimensions {
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn calculate(logical_width: u32, logical_height: u32, scale_factor: f32) -> Self {
        let physical_width = (logical_width as f32 * scale_factor).round() as u32;
        let physical_height = (logical_height as f32 * scale_factor).round() as u32;
        let buffer_scale = scale_factor.round() as i32;

        Self {
            logical_width,
            logical_height,
            physical_width,
            physical_height,
            buffer_scale,
        }
    }
}
