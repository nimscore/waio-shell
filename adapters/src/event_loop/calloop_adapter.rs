use crate::wayland::{surfaces::surface_state::WindowState, shell_adapter::WaylandWindowingSystem};
use crate::{
    errors::Result,
    platform::calloop::{EventSource, InsertError, RegistrationToken},
};
use slint_interpreter::ComponentInstance;
use std::result::Result as StdResult;

pub struct SystemAdapter {
    inner: WaylandWindowingSystem,
}

impl SystemAdapter {
    #[must_use]
    pub fn new(inner: WaylandWindowingSystem) -> Self {
        Self { inner }
    }

    #[must_use]
    pub fn event_loop_handle(&self) -> EventLoopAdapter {
        EventLoopAdapter {
            inner_system: std::ptr::addr_of!(self.inner),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.run()
    }

    pub const fn component_instance(&self) -> &ComponentInstance {
        self.inner.component_instance()
    }
}

pub struct EventLoopAdapter {
    inner_system: *const WaylandWindowingSystem,
}

unsafe impl Send for EventLoopAdapter {}
unsafe impl Sync for EventLoopAdapter {}

impl EventLoopAdapter {
    pub fn insert_source_with_adapter<S, F, R>(
        &self,
        source: S,
        mut callback: F,
    ) -> StdResult<RegistrationToken, InsertError<S>>
    where
        S: EventSource<Ret = R> + 'static,
        F: FnMut(S::Event, &mut S::Metadata, RuntimeStateAdapter) -> R + 'static,
    {
        let inner_system = unsafe { &*self.inner_system };
        let loop_handle = inner_system.event_loop_handle();

        loop_handle.insert_source(source, move |event, metadata, window_state| {
            let runtime_state = RuntimeStateAdapter {
                window_state: std::ptr::addr_of_mut!(*window_state),
            };
            callback(event, metadata, runtime_state)
        })
    }
}

pub struct RuntimeStateAdapter {
    window_state: *mut WindowState,
}

impl RuntimeStateAdapter {
    #[must_use]
    pub fn component_instance(&self) -> &ComponentInstance {
        let window_state = unsafe { &*self.window_state };
        window_state.component_instance()
    }

    pub fn render_frame_if_dirty(&self) -> Result<()> {
        let window_state = unsafe { &*self.window_state };
        window_state.render_frame_if_dirty()
    }
}
