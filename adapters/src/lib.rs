#![allow(clippy::pub_use)]

pub mod errors;
pub mod rendering;
pub mod wayland;

pub use rendering::femtovg::popup_window::PopupWindow;
pub use rendering::slint_integration::platform::{
    clear_popup_config, close_current_popup, get_popup_config, set_popup_config,
};

pub mod platform {
    pub use slint;
    pub use slint_interpreter;

    pub mod calloop {
        pub use smithay_client_toolkit::reexports::calloop::channel;
        pub use smithay_client_toolkit::reexports::calloop::generic::Generic;
        pub use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
        pub use smithay_client_toolkit::reexports::calloop::{
            EventSource, InsertError, Interest, Mode, PostAction, RegistrationToken,
        };
    }
}
