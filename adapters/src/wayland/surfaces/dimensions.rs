use layer_shika_domain::surface_dimensions::SurfaceDimensions;
use slint::PhysicalSize;

pub trait SurfaceDimensionsExt {
    fn logical_size(&self) -> PhysicalSize;
    fn physical_size(&self) -> PhysicalSize;
}

impl SurfaceDimensionsExt for SurfaceDimensions {
    fn logical_size(&self) -> PhysicalSize {
        PhysicalSize::new(self.logical_width, self.logical_height)
    }

    fn physical_size(&self) -> PhysicalSize {
        PhysicalSize::new(self.physical_width, self.physical_height)
    }
}
