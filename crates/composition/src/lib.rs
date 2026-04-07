#![allow(clippy::pub_use)]

mod event_loop;
mod layer_surface;
mod lock_selection;
mod popup;
mod popup_builder;
mod selection;
mod selector;
mod session_lock;
mod shell;
mod shell_config;
mod shell_runtime;
mod surface_registry;
mod system;
pub mod value_conversion;

use std::result::Result as StdResult;

pub use event_loop::{EventLoopHandle, ShellEventLoop};
pub use waio_shell_adapters::PopupWindow;
use waio_shell_adapters::errors::WaioShellError;
pub use waio_shell_adapters::platform::{slint, slint_interpreter};
pub use waio_shell_domain::entities::output_registry::OutputRegistry;
use waio_shell_domain::errors::DomainError;
pub use waio_shell_domain::prelude::AnchorStrategy;
pub use waio_shell_domain::value_objects::anchor::AnchorEdges;
pub use waio_shell_domain::value_objects::handle::{Handle, PopupHandle, SurfaceHandle};
pub use waio_shell_domain::value_objects::keyboard_interactivity::KeyboardInteractivity;
pub use waio_shell_domain::value_objects::layer::Layer;
pub use waio_shell_domain::value_objects::output_handle::OutputHandle;
pub use waio_shell_domain::value_objects::output_info::{OutputGeometry, OutputInfo};
pub use waio_shell_domain::value_objects::output_policy::OutputPolicy;
pub use waio_shell_domain::value_objects::output_target::OutputTarget;
pub use waio_shell_domain::value_objects::popup_behavior::{
    ConstraintAdjustment, OutputMigrationPolicy, PopupBehavior,
};
pub use waio_shell_domain::value_objects::popup_config::PopupConfig;
pub use waio_shell_domain::value_objects::popup_position::{
    Alignment, AnchorPoint, Offset, PopupPosition,
};
pub use waio_shell_domain::value_objects::popup_size::PopupSize;
pub use waio_shell_domain::value_objects::surface_instance_id::SurfaceInstanceId;
pub use layer_surface::{LayerSurfaceHandle, ShellSurfaceConfigHandler};
pub use lock_selection::LockSelection;
pub use popup::PopupShell;
pub use popup_builder::{Bound, PopupBuilder, Unbound};
pub use selection::{PropertyError, Selection, SelectionResult};
pub use selector::{Output, Selector, Surface, SurfaceInfo};
pub use session_lock::{SessionLock, SessionLockBuilder};
pub use shell::{
    DEFAULT_COMPONENT_NAME, Shell, ShellBuilder, ShellEventContext, SurfaceConfigBuilder,
};
pub use shell_config::{CompiledUiSource, ShellConfig, SurfaceComponentConfig};
pub use shell_runtime::{DEFAULT_SURFACE_NAME, ShellRuntime};
pub use surface_registry::{SurfaceDefinition, SurfaceEntry, SurfaceMetadata, SurfaceRegistry};
pub use system::{
    CallbackContext, EventDispatchContext, RuntimeSurfaceConfigBuilder, ShellControl,
    SurfaceControlHandle, SurfaceTarget,
};
pub use value_conversion::IntoValue;

pub(crate) mod logger {
    #[cfg(all(feature = "log", feature = "tracing"))]
    compile_error!("Cannot use both logging backend at one time");

    #[cfg(feature = "log")]
    pub use log::{debug, error, info, warn};
    #[cfg(feature = "tracing")]
    pub use tracing::{debug, error, info, warn};
}

pub mod calloop {
    pub use waio_shell_adapters::platform::calloop::*;
}

/// Result type alias using waio-shell's Error
pub type Result<T> = StdResult<T, Error>;

/// Error types for waio-shell operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Adapter error: {0}")]
    Adapter(#[from] WaioShellError),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("App has been dropped")]
    SystemDropped,

    #[error("Invalid lock state '{current}' for operation '{operation}'")]
    InvalidState { current: String, operation: String },

    #[error("Protocol '{protocol}' not available on this compositor")]
    ProtocolNotAvailable { protocol: String },
}

pub mod prelude {
    pub use waio_shell_adapters::platform::wayland::Anchor;
    pub use waio_shell_domain::prelude::{
        LogicalPosition, LogicalRect, LogicalSize, Margins, PhysicalSize, ScaleFactor,
        SurfaceConfig, SurfaceDimension, UiSource,
    };

    pub use crate::calloop::{Generic, Interest, Mode, PostAction, RegistrationToken, Timer};
    pub use crate::{
        AnchorEdges, AnchorStrategy, CompiledUiSource, DEFAULT_COMPONENT_NAME,
        DEFAULT_SURFACE_NAME, EventDispatchContext, EventLoopHandle, Handle, IntoValue,
        KeyboardInteractivity, Layer, LayerSurfaceHandle, LockSelection, Output, OutputGeometry,
        OutputHandle, OutputInfo, OutputPolicy, OutputRegistry, PopupBuilder, PopupConfig,
        PopupHandle, PopupPosition, PopupShell, PopupSize, PopupWindow, PropertyError, Result,
        Selection, SelectionResult, Selector, SessionLock, SessionLockBuilder, Shell, ShellBuilder,
        ShellConfig, ShellControl, ShellEventContext, ShellEventLoop, ShellRuntime,
        ShellSurfaceConfigHandler, Surface, SurfaceComponentConfig, SurfaceConfigBuilder,
        SurfaceControlHandle, SurfaceDefinition, SurfaceEntry, SurfaceHandle, SurfaceInfo,
        SurfaceMetadata, SurfaceRegistry, slint, slint_interpreter,
    };
}
