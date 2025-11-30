//! Prelude module re-exporting all public API types
//!
//! Import this module to get access to the most commonly used types:
//!
//! ```rust
//! use layer_shika::prelude::*;
//! ```

#![allow(clippy::pub_use)]

// Core API types
pub use crate::{
    App, Error, EventContext, EventLoopHandle, LayerShika, PopupWindow, Result, ShellControl,
};

// Domain value objects
pub use crate::{
    AnchorEdges, KeyboardInteractivity, Layer, OutputGeometry, OutputHandle, OutputInfo,
    OutputPolicy, OutputRegistry, PopupHandle, PopupPlacement, PopupPositioningMode, PopupRequest,
    PopupSize,
};

// Event loop types
pub use crate::calloop;

// UI framework re-exports
pub use crate::{slint, slint_interpreter};
