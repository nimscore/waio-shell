#![allow(clippy::pub_use)]

pub mod errors;
pub(crate) mod rendering;
pub(crate) mod wayland;

pub use rendering::femtovg::popup_window::PopupWindow;

pub use wayland::{
    config::WaylandWindowConfig,
    shell_adapter::WaylandWindowingSystem,
    surfaces::{
        popup_manager::{PopupId, PopupManager},
        surface_state::WindowState,
    },
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
