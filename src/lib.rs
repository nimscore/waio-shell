//! layer-shika: A Wayland layer shell library with Slint UI integration
//!
//! This crate provides a high-level API for creating Wayland layer shell surfaces
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
//! - [`window`] – Surface configuration, layers, anchors, and popup types
//! - [`output`] – Output (monitor) info, geometry, and policies
//! - [`event`] – Event loop handles and contexts
//! - [`slint_integration`] – Slint framework re-exports and wrappers
//! - [`calloop`] – Event loop types for custom event sources
//!
//! # Quick Start (Fluent Builder)
//!
//! Single-surface use case with the fluent builder API:
//!
//! ```rust,no_run
//! use layer_shika::prelude::*;
//!
//! Shell::from_file("ui/bar.slint")
//!     .surface("Main")
//!         .height(42)
//!         .anchor(AnchorEdges::top_bar())
//!         .exclusive_zone(42)
//!     .build()?
//!     .run()?;
//! # Ok::<(), layer_shika::Error>(())
//! ```
//!
//! # Declarative Configuration
//!
//! For reusable, programmatically generated, or externally sourced configurations:
//!
//! ```rust,no_run
//! use layer_shika::prelude::*;
//!
//! let config = ShellConfig {
//!     ui_source: UiSource::file("ui/bar.slint"),
//!     surfaces: vec![
//!         SurfaceComponentConfig::with_config("Bar", SurfaceConfig {
//!             dimensions: SurfaceDimension::new(0, 42),
//!             anchor: AnchorEdges::top_bar(),
//!             exclusive_zone: 42,
//!             ..Default::default()
//!         }),
//!     ],
//! };
//!
//! Shell::from_config(config)?.run()?;
//! # Ok::<(), layer_shika::Error>(())
//! ```
//!
//! # Multi-Surface Shell
//!
//! Same API naturally extends to multiple surfaces:
//!
//! ```rust,no_run
//! use layer_shika::prelude::*;
//!
//! Shell::from_file("ui/shell.slint")
//!     .surface("TopBar")
//!         .height(42)
//!         .anchor(AnchorEdges::top_bar())
//!     .surface("Dock")
//!         .height(64)
//!         .anchor(AnchorEdges::bottom_bar())
//!     .build()?
//!     .run()?;
//! # Ok::<(), layer_shika::Error>(())
//! ```
//!
//! # Pre-compiled Slint
//!
//! For explicit compilation control:
//!
//! ```rust,no_run
//! use layer_shika::prelude::*;
//!
//! let compilation = Shell::compile_file("ui/shell.slint")?;
//!
//! Shell::from_compilation(compilation)
//!     .surface("TopBar")
//!         .output_policy(OutputPolicy::AllOutputs)
//!         .height(42)
//!     .surface("Dock")
//!         .output_policy(OutputPolicy::PrimaryOnly)
//!         .height(64)
//!     .build()?
//!     .run()?;
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
    CompiledUiSource, DEFAULT_COMPONENT_NAME, DEFAULT_SURFACE_NAME, LayerSurfaceHandle, Shell,
    ShellBuilder, ShellConfig, ShellControl, ShellEventContext, ShellEventLoopHandle, ShellRuntime,
    ShellSurfaceConfigHandler, ShellSurfaceHandle, SingleSurfaceShell, SurfaceComponentConfig,
    SurfaceConfigBuilder, SurfaceDefinition,
};

pub use window::{
    AnchorEdges, AnchorStrategy, KeyboardInteractivity, Layer, PopupHandle, PopupPlacement,
    PopupPositioningMode, PopupRequest, PopupSize,
};

pub use output::{OutputGeometry, OutputHandle, OutputInfo, OutputPolicy, OutputRegistry};

pub use event::{EventContext, EventLoopHandle};

pub use slint_integration::{PopupWindow, slint, slint_interpreter};

pub mod calloop {
    pub use layer_shika_composition::calloop::*;
}
