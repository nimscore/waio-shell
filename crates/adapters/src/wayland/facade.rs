use crate::errors::Result;
use crate::wayland::shell_adapter::WaylandWindowingSystem;
use slint_interpreter::ComponentInstance;

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

    pub fn component_instance(&self) -> Result<&ComponentInstance> {
        self.inner.component_instance()
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.run()
    }
}
