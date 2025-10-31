#![allow(clippy::pub_use)]

pub mod errors;
pub mod rendering;
pub mod wayland;

pub use rendering::femtovg::popup_window::PopupWindow;
pub use rendering::slint_integration::platform::{
    clear_popup_position_override, close_current_popup, get_popup_position_override,
    set_popup_position_override,
};

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
