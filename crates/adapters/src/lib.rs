#![allow(clippy::pub_use)]

pub mod errors;
pub(crate) mod rendering;
pub(crate) mod wayland;

pub use rendering::femtovg::popup_window::PopupWindow;

pub use wayland::config::WaylandWindowConfig;
pub use wayland::facade::{PopupManagerFacade, RuntimeStateFacade, WindowingSystemFacade};
pub use wayland::shell_adapter::WaylandWindowingSystem;
pub use wayland::surfaces::app_state::AppState;
pub use wayland::surfaces::popup_manager::PopupManager;
pub use wayland::surfaces::surface_state::WindowState;

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
