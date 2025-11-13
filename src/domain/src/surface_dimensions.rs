use crate::dimensions::{LogicalSize, PhysicalSize, ScaleFactor};
use crate::errors::Result;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceDimensions {
    logical: LogicalSize,
    physical: PhysicalSize,
    scale_factor: ScaleFactor,
}

impl SurfaceDimensions {
    #[allow(clippy::cast_precision_loss)]
    pub fn calculate(
        logical_width: u32,
        logical_height: u32,
        scale_factor: f32,
    ) -> Result<Self> {
        let logical = LogicalSize::new(logical_width as f32, logical_height as f32)?;
        let scale = ScaleFactor::new(scale_factor)?;
        let physical = scale.to_physical(logical);

        Ok(Self {
            logical,
            physical,
            scale_factor: scale,
        })
    }

    pub const fn logical_size(&self) -> LogicalSize {
        self.logical
    }

    pub const fn physical_size(&self) -> PhysicalSize {
        self.physical
    }

    pub const fn scale_factor(&self) -> ScaleFactor {
        self.scale_factor
    }

    pub fn buffer_scale(&self) -> i32 {
        self.scale_factor.buffer_scale()
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn logical_width(&self) -> u32 {
        self.logical.width() as u32
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn logical_height(&self) -> u32 {
        self.logical.height() as u32
    }

    pub fn physical_width(&self) -> u32 {
        self.physical.width()
    }

    pub fn physical_height(&self) -> u32 {
        self.physical.height()
    }
}
