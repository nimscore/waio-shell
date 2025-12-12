//! Prelude module re-exporting all public API types
//!
//! Import this module to get access to the most commonly used types:
//!
//! ```rust
//! use layer_shika::prelude::*;
//! ```

#![allow(clippy::pub_use)]

pub use crate::shell::{
    CompiledUiSource, DEFAULT_COMPONENT_NAME, DEFAULT_SURFACE_NAME, LayerSurfaceHandle, Output,
    Selection, Selector, Shell, ShellBuilder, ShellConfig, ShellControl, ShellEventContext,
    ShellRuntime, ShellSurfaceConfigHandler, Surface, SurfaceComponentConfig, SurfaceConfigBuilder,
    SurfaceDefinition, SurfaceInfo,
};

pub use crate::window::{
    AnchorEdges, AnchorStrategy, KeyboardInteractivity, Layer, PopupHandle, PopupPlacement,
    PopupPositioningMode, PopupRequest, PopupSize,
};

pub use crate::output::{OutputGeometry, OutputHandle, OutputInfo, OutputPolicy, OutputRegistry};

pub use crate::event::{EventDispatchContext, EventLoopHandle, ShellEventLoop};

pub use crate::slint_integration::{PopupWindow, slint, slint_interpreter};

pub use crate::{CallbackContext, Error, Handle, Result, SurfaceHandle, SurfaceInstanceId, SurfaceTarget};

pub use layer_shika_composition::prelude::{
    Anchor, LogicalSize, Margins, PhysicalSize, ScaleFactor, SurfaceConfig, SurfaceDimension,
    UiSource,
};

pub use crate::calloop;
