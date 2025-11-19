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
    Error, EventLoopHandle, LayerShika, PopupWindow, Result, ShellContext, SlintCallbackContract,
    SlintCallbackNames, WindowingSystem,
};

// Domain value objects
pub use crate::{
    AnchorEdges, KeyboardInteractivity, Layer, OutputGeometry, OutputHandle, OutputInfo,
    OutputPolicy, OutputRegistry, PopupAt, PopupHandle, PopupPositioningMode, PopupRequest, PopupSize,
};

// Event loop types
pub use crate::calloop;

// UI framework re-exports
pub use crate::{slint, slint_interpreter};
