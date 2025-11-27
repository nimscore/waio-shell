#![allow(clippy::pub_use)]

mod builder;
mod popup_builder;
mod system;
mod value_conversion;

use layer_shika_adapters::errors::LayerShikaError;
use layer_shika_domain::errors::DomainError;
use std::result::Result as StdResult;

pub use builder::LayerShika;
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
    PopupAt, PopupHandle, PopupRequest, PopupSize,
};
pub use popup_builder::PopupBuilder;
pub use system::{App, EventLoopHandle, ShellContext, ShellControl};

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
        AnchorEdges, AnchorStrategy, App, EventLoopHandle, KeyboardInteractivity, Layer,
        LayerShika, OutputGeometry, OutputHandle, OutputInfo, OutputPolicy, OutputRegistry,
        PopupAt, PopupBuilder, PopupHandle, PopupPositioningMode, PopupRequest, PopupSize,
        PopupWindow, Result, ShellContext, ShellControl,
    };

    pub use crate::calloop::{Generic, Interest, Mode, PostAction, RegistrationToken, Timer};

    pub use crate::{slint, slint_interpreter};
}
