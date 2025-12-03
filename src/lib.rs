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
//! # Module Organization
//!
//! The API is organized into conceptual facets:
//!
//! - [`shell`] – Main runtime and shell composition types
//! - [`window`] – Window configuration, layers, anchors, and popup types
//! - [`output`] – Output (monitor) info, geometry, and policies
//! - [`event`] – Event loop handles and contexts
//! - [`slint_integration`] – Slint framework re-exports and wrappers
//! - [`calloop`] – Event loop types for custom event sources
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use layer_shika::prelude::*;
//!
//! LayerShika::from_file("ui/main.slint")?
//!     .height(42)
//!     .anchor(AnchorEdges::top_bar())
//!     .exclusive_zone(42)
//!     .run()?;
//! # Ok::<(), layer_shika::Error>(())
//! ```
//!
//! # Multi-Window Shell
//!
//! For multi-window shell applications:
//!
//! ```rust,no_run
//! use layer_shika::prelude::*;
//! use std::rc::Rc;
//!
//! // Load Slint file with multiple shell window components
//! let compilation_result = Rc::new(/* ... */);
//!
//! // Create shell with typed WindowConfig
//! let shell = ShellComposition::new()
//!     .with_compilation_result(compilation_result)
//!     .with_window("TopBar", WindowConfig::default())
//!     .build()?;
//!
//! shell.run()?;
//! # Ok::<(), layer_shika::Error>(())
//! ```

#![allow(clippy::pub_use)]

pub mod prelude;

pub mod event;
pub mod output;
pub mod shell;
pub mod slint_integration;
pub mod window;

pub use layer_shika_composition::{Error, Result};

pub use shell::{
    DEFAULT_WINDOW_NAME, LayerShika, LayerSurfaceHandle, Shell, ShellComposition, ShellControl,
    ShellEventContext, ShellRuntime, ShellWindowConfigHandler, ShellWindowDefinition,
    ShellWindowHandle, SingleWindowShell,
};

pub use window::{
    AnchorEdges, AnchorStrategy, KeyboardInteractivity, Layer, PopupHandle, PopupPlacement,
    PopupPositioningMode, PopupRequest, PopupSize,
};

pub use output::{OutputGeometry, OutputHandle, OutputInfo, OutputPolicy, OutputRegistry};

pub use event::{EventContext, EventLoopHandle, ShellEventLoopHandle};

pub use slint_integration::{PopupWindow, slint, slint_interpreter};

pub mod calloop {
    pub use layer_shika_composition::calloop::*;
}
