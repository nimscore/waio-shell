use crate::errors::Result;
use crate::wayland::shell_adapter::WaylandWindowingSystem;
use crate::wayland::surfaces::popup_manager::PopupManager;
use crate::wayland::surfaces::surface_state::WindowState;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::ports::windowing::RuntimeStatePort;
use slint_interpreter::ComponentInstance;
use std::rc::Rc;
use std::result::Result as StdResult;

pub struct WindowingSystemFacade {
    inner: WaylandWindowingSystem,
}

impl WindowingSystemFacade {
    pub fn new(inner: WaylandWindowingSystem) -> Self {
        Self { inner }
    }

    pub fn inner_ref(&self) -> &WaylandWindowingSystem {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut WaylandWindowingSystem {
        &mut self.inner
    }

    pub fn component_instance(&self) -> &ComponentInstance {
        self.inner.component_instance()
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.run()
    }
}

pub struct RuntimeStateFacade<'a> {
    window_state: &'a mut WindowState,
}

impl<'a> RuntimeStateFacade<'a> {
    pub fn new(window_state: &'a mut WindowState) -> Self {
        Self { window_state }
    }

    pub fn popup_manager(&self) -> Option<Rc<PopupManager>> {
        self.window_state.popup_manager().cloned()
    }

    pub fn component_instance(&self) -> &ComponentInstance {
        self.window_state.component_instance()
    }

    pub fn window_state(&self) -> &WindowState {
        self.window_state
    }

    pub fn window_state_mut(&mut self) -> &mut WindowState {
        self.window_state
    }
}

impl RuntimeStatePort for RuntimeStateFacade<'_> {
    fn render_frame_if_dirty(&mut self) -> StdResult<(), DomainError> {
        self.window_state
            .render_frame_if_dirty()
            .map_err(|e| DomainError::Adapter {
                source: Box::new(e),
            })
    }
}

pub struct PopupManagerFacade {
    inner: Rc<PopupManager>,
}

impl PopupManagerFacade {
    pub fn new(inner: Rc<PopupManager>) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &Rc<PopupManager> {
        &self.inner
    }
}
