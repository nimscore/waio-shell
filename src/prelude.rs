//! Prelude module re-exporting all public API types
//!
//! Import this module to get access to the most commonly used types:
//!
//! ```rust
//! use layer_shika::prelude::*;
//! ```

#![allow(clippy::pub_use)]

pub use crate::{
    App, Error, EventContext, EventLoopHandle, LayerShika, PopupWindow, Result, ShellControl,
};

pub use crate::{
    LayerSurfaceHandle, Shell, ShellComposition, ShellEventContext, ShellEventLoopHandle,
    ShellWindowConfigHandler, ShellWindowDefinition, ShellWindowHandle,
};

pub use crate::{
    AnchorEdges, KeyboardInteractivity, Layer, OutputGeometry, OutputHandle, OutputInfo,
    OutputPolicy, OutputRegistry, PopupHandle, PopupPlacement, PopupPositioningMode, PopupRequest,
    PopupSize,
};

pub use layer_shika_composition::prelude::{
    Anchor, Margins, ScaleFactor, WindowConfig, WindowDimension,
};

pub use crate::calloop;

pub use crate::{slint, slint_interpreter};
