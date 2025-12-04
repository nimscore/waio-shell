#![allow(clippy::pub_use)]

pub use crate::config::SurfaceConfig;
pub use crate::dimensions::{
    LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize, ScaleFactor,
};
pub use crate::entities::output_registry::OutputRegistry;
pub use crate::errors::{DomainError, Result};
pub use crate::surface_dimensions::SurfaceDimensions;
pub use crate::value_objects::anchor::AnchorEdges;
pub use crate::value_objects::anchor_strategy::AnchorStrategy;
pub use crate::value_objects::dimensions::{PopupDimensions, SurfaceDimension};
pub use crate::value_objects::keyboard_interactivity::KeyboardInteractivity;
pub use crate::value_objects::layer::Layer;
pub use crate::value_objects::margins::Margins;
pub use crate::value_objects::output_handle::OutputHandle;
pub use crate::value_objects::output_info::{OutputGeometry, OutputInfo};
pub use crate::value_objects::output_policy::OutputPolicy;
