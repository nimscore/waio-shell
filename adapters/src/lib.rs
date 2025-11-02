#![allow(clippy::pub_use)]

pub mod errors;
pub mod rendering;
pub mod wayland;

pub use rendering::femtovg::popup_window::PopupWindow;
pub use rendering::slint_integration::platform::close_current_popup;

pub mod platform {
    pub use slint;
    pub use slint_interpreter;

    pub mod calloop {
        pub use smithay_client_toolkit::reexports::calloop::channel;
        pub use smithay_client_toolkit::reexports::calloop::{
            EventSource, InsertError, PostAction, RegistrationToken,
        };
    }
}
