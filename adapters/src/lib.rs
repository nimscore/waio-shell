#![allow(clippy::pub_use)]

pub mod errors;
pub mod event_loop;
pub mod rendering;
pub mod wayland;

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
