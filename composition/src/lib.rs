#![allow(clippy::pub_use)]

pub mod builder;
pub mod system;

use layer_shika_adapters::errors::LayerShikaError;
use layer_shika_domain::errors::DomainError;
use std::result::Result as StdResult;

pub use builder::LayerShika;
pub use layer_shika_adapters::PopupWindow;
pub use layer_shika_adapters::close_current_popup;
pub use layer_shika_adapters::platform::{calloop, slint, slint_interpreter};
pub use layer_shika_adapters::{
    clear_popup_position_override, get_popup_position_override, set_popup_position_override,
    clear_popup_size_override, get_popup_size_override, set_popup_size_override,
};
pub use layer_shika_domain::value_objects::anchor::AnchorEdges;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Adapter error: {0}")]
    Adapter(#[from] LayerShikaError),

    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("WindowingSystem has been dropped")]
    SystemDropped,

    #[error("Cannot run while EventLoopHandle exists")]
    EventLoopHandleExists,
}
