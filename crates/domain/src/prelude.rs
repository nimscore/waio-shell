#![allow(clippy::pub_use)]

pub use crate::config::SurfaceConfig;
pub use crate::dimensions::{
    LogicalPosition, LogicalRect, LogicalSize, PhysicalPosition, PhysicalSize, ScaleFactor,
};
pub use crate::entities::output_registry::OutputRegistry;
pub use crate::entities::popup_tree::PopupTree;
pub use crate::errors::{DomainError, Result};
pub use crate::surface_dimensions::SurfaceDimensions;
pub use crate::value_objects::anchor::AnchorEdges;
pub use crate::value_objects::anchor_strategy::AnchorStrategy;
pub use crate::value_objects::dimensions::{PopupDimensions, SurfaceDimension};
pub use crate::value_objects::handle::{Handle, OutputHandle, PopupHandle, SurfaceHandle};
pub use crate::value_objects::keyboard_interactivity::KeyboardInteractivity;
pub use crate::value_objects::layer::Layer;
pub use crate::value_objects::lock_config::LockConfig;
pub use crate::value_objects::lock_state::LockState;
pub use crate::value_objects::margins::Margins;
pub use crate::value_objects::output_info::{OutputGeometry, OutputInfo};
pub use crate::value_objects::output_policy::OutputPolicy;
pub use crate::value_objects::output_target::OutputTarget;
pub use crate::value_objects::popup_behavior::{
    ConstraintAdjustment, OutputMigrationPolicy, PopupBehavior,
};
pub use crate::value_objects::popup_config::PopupConfig;
pub use crate::value_objects::popup_position::{Alignment, AnchorPoint, Offset, PopupPosition};
pub use crate::value_objects::popup_size::PopupSize;
pub use crate::value_objects::ui_source::UiSource;
