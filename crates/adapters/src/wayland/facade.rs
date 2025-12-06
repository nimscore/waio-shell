use crate::errors::Result;
use crate::wayland::shell_adapter::WaylandShellSystem;
use slint_interpreter::ComponentInstance;

pub struct ShellSystemFacade {
    inner: WaylandShellSystem,
}

impl ShellSystemFacade {
    pub fn new(inner: WaylandShellSystem) -> Self {
        Self { inner }
    }

    pub fn inner_ref(&self) -> &WaylandShellSystem {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut WaylandShellSystem {
        &mut self.inner
    }

    pub fn component_instance(&self) -> Result<&ComponentInstance> {
        self.inner.component_instance()
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.run()
    }
}
