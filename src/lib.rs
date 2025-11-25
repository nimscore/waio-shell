//! layer-shika: A Wayland layer shell library with Slint UI integration
//!
//! This crate provides a high-level API for creating Wayland layer shell windows
//! with Slint-based user interfaces. It's built on a clean architecture with three
//! internal layers (domain, adapters, composition), but users should only depend on
//! this root crate.
//!
//! # Architecture Note
//!
//! layer-shika is internally organized as a Cargo workspace with three implementation
//! crates:
//! - `layer-shika-domain`: Core domain models and business logic
//! - `layer-shika-adapters`: Wayland and rendering implementations
//! - `layer-shika-composition`: Public API composition layer
//!
//! **Users should never import from these internal crates directly.** This allows
//! the internal architecture to evolve without breaking semver guarantees on the
//! public API.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use layer_shika::prelude::*;
//!
//! LayerShika::from_file("ui/main.slint", Some("AppWindow"))?
//!     .with_height(42)
//!     .with_anchor(AnchorEdges::top_bar())
//!     .with_exclusive_zone(42)
//!     .run()?;
//! # Ok::<(), layer_shika::Error>(())
//! ```
//!
//! # Re-exports
//!
//! This crate re-exports commonly needed types from its dependencies:
//! - [`slint`]: The Slint UI framework (compiled API)
//! - [`slint_interpreter`]: Runtime Slint component loading
//! - [`calloop`]: Event loop types for custom event sources

#![allow(clippy::pub_use)]

pub mod prelude;

pub use layer_shika_composition::{
    AnchorEdges, App, Error, EventLoopHandle, KeyboardInteractivity, Layer, LayerShika,
    OutputGeometry, OutputHandle, OutputInfo, OutputPolicy, OutputRegistry, PopupAt, PopupHandle,
    PopupPositioningMode, PopupRequest, PopupSize, PopupWindow, Result, ShellContext, ShellControl,
};

pub use layer_shika_composition::{slint, slint_interpreter};

/// Re-exported calloop types for event loop integration
///
/// These types allow users to register custom event sources on the
/// layer-shika event loop via [`EventLoopHandle`].
pub mod calloop {
    pub use layer_shika_composition::calloop::*;
}
