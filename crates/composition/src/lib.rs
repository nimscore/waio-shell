#![allow(clippy::pub_use)]

mod event_loop;
mod layer_surface;
mod popup_builder;
mod shell;
mod shell_config;
mod shell_runtime;
mod system;
pub mod value_conversion;

use layer_shika_adapters::errors::LayerShikaError;
use layer_shika_domain::errors::DomainError;
use std::result::Result as StdResult;

pub use layer_shika_adapters::PopupWindow;
pub use layer_shika_adapters::platform::{slint, slint_interpreter};
pub use layer_shika_domain::entities::output_registry::OutputRegistry;
pub use layer_shika_domain::prelude::AnchorStrategy;
pub use layer_shika_domain::value_objects::anchor::AnchorEdges;
pub use layer_shika_domain::value_objects::keyboard_interactivity::KeyboardInteractivity;
pub use layer_shika_domain::value_objects::layer::Layer;
pub use layer_shika_domain::value_objects::output_handle::OutputHandle;
pub use layer_shika_domain::value_objects::output_info::{OutputGeometry, OutputInfo};
pub use layer_shika_domain::value_objects::output_policy::OutputPolicy;
pub use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
pub use layer_shika_domain::value_objects::popup_request::{
    PopupHandle, PopupPlacement, PopupRequest, PopupSize,
};
pub use popup_builder::PopupBuilder;
pub use shell_runtime::{DEFAULT_SURFACE_NAME, ShellRuntime};
pub use system::{EventContext, EventLoopHandle, ShellControl, SingleSurfaceShell};
pub use value_conversion::IntoValue;

pub use layer_surface::{LayerSurfaceHandle, ShellSurfaceConfigHandler, ShellSurfaceHandle};

pub use shell::{
    DEFAULT_COMPONENT_NAME, Shell, ShellBuilder, ShellEventContext, ShellEventLoopHandle,
    SurfaceConfigBuilder, SurfaceDefinition,
};

pub use shell_config::{CompiledUiSource, ShellConfig, SurfaceComponentConfig};

pub mod calloop {
    pub use layer_shika_adapters::platform::calloop::{
        Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer, channel,
    };
}

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Adapter error: {0}")]
    Adapter(#[from] LayerShikaError),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("App has been dropped")]
    SystemDropped,
}

pub mod prelude {
    pub use crate::{
        AnchorEdges, AnchorStrategy, CompiledUiSource, DEFAULT_COMPONENT_NAME,
        DEFAULT_SURFACE_NAME, EventContext, EventLoopHandle, IntoValue, KeyboardInteractivity,
        Layer, LayerSurfaceHandle, OutputGeometry, OutputHandle, OutputInfo, OutputPolicy,
        OutputRegistry, PopupBuilder, PopupHandle, PopupPlacement, PopupPositioningMode,
        PopupRequest, PopupSize, PopupWindow, Result, Shell, ShellBuilder, ShellConfig,
        ShellControl, ShellEventContext, ShellEventLoopHandle, ShellRuntime,
        ShellSurfaceConfigHandler, ShellSurfaceHandle, SingleSurfaceShell, SurfaceComponentConfig,
        SurfaceConfigBuilder, SurfaceDefinition,
    };

    pub use crate::calloop::{Generic, Interest, Mode, PostAction, RegistrationToken, Timer};

    pub use crate::{slint, slint_interpreter};

    pub use layer_shika_domain::prelude::{
        LogicalSize, Margins, PhysicalSize, ScaleFactor, SurfaceConfig, SurfaceDimension, UiSource,
    };

    pub use layer_shika_adapters::platform::wayland::Anchor;
}
