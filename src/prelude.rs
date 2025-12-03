//! Prelude module re-exporting all public API types
//!
//! Import this module to get access to the most commonly used types:
//!
//! ```rust
//! use layer_shika::prelude::*;
//! ```

#![allow(clippy::pub_use)]

pub use crate::shell::{
    DEFAULT_WINDOW_NAME, LayerShika, LayerSurfaceHandle, Shell, ShellComposition, ShellControl,
    ShellEventContext, ShellEventLoopHandle, ShellRuntime, ShellWindowConfigHandler,
    ShellWindowDefinition, ShellWindowHandle, SingleWindowShell,
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
