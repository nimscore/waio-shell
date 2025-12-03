//! Prelude module re-exporting all public API types
//!
//! Import this module to get access to the most commonly used types:
//!
//! ```rust
//! use layer_shika::prelude::*;
//! ```

#![allow(clippy::pub_use)]

pub use crate::shell::{
    DEFAULT_COMPONENT_NAME, DEFAULT_WINDOW_NAME, LayerShika, LayerShikaEventContext,
    LayerShikaEventLoopHandle, LayerSurfaceHandle, Runtime, Shell, ShellBuilder, ShellControl,
    ShellEventContext, ShellEventLoopHandle, ShellRuntime, ShellWindowConfigHandler,
    ShellWindowHandle, SingleWindowShell, WindowConfigBuilder, WindowDefinition,
};

pub use crate::window::{
    AnchorEdges, AnchorStrategy, KeyboardInteractivity, Layer, PopupHandle, PopupPlacement,
    PopupPositioningMode, PopupRequest, PopupSize,
};

pub use crate::output::{OutputGeometry, OutputHandle, OutputInfo, OutputPolicy, OutputRegistry};

pub use crate::event::{EventContext, EventLoopHandle};

pub use crate::slint_integration::{PopupWindow, slint, slint_interpreter};

pub use crate::{Error, Result};

pub use layer_shika_composition::prelude::{
    Anchor, LogicalSize, Margins, PhysicalSize, ScaleFactor, WindowConfig, WindowDimension,
};

pub use crate::calloop;
