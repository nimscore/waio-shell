use crate::Result;
use layer_shika_adapters::platform::calloop::{EventSource, InsertError, RegistrationToken};
use layer_shika_adapters::platform::slint_interpreter::{ComponentDefinition, ComponentInstance};
use layer_shika_adapters::wayland::{
    config::WaylandWindowConfig,
    shell_adapter::WaylandWindowingSystem,
};
use layer_shika_adapters::event_loop::calloop_adapter::{
    EventLoopAdapter, RuntimeStateAdapter, SystemAdapter,
};
use layer_shika_domain::config::WindowConfig;
use std::result::Result as StdResult;

pub struct EventLoopHandle {
    adapter: EventLoopAdapter,
}

impl EventLoopHandle {
    pub fn insert_source<S, F, R>(
        &self,
        source: S,
        mut callback: F,
    ) -> StdResult<RegistrationToken, InsertError<S>>
    where
        S: EventSource<Ret = R> + 'static,
        F: FnMut(S::Event, &mut S::Metadata, &mut RuntimeState) -> R + 'static,
    {
        self.adapter
            .insert_source_with_adapter(source, move |event, metadata, adapter| {
                let mut runtime_state = RuntimeState { adapter };
                callback(event, metadata, &mut runtime_state)
            })
    }
}

pub struct RuntimeState {
    adapter: RuntimeStateAdapter,
}

impl RuntimeState {
    #[must_use]
    pub fn component_instance(&self) -> &ComponentInstance {
        self.adapter.component_instance()
    }

    pub fn render_frame_if_dirty(&self) -> Result<()> {
        Ok(self.adapter.render_frame_if_dirty()?)
    }
}

pub struct WindowingSystem {
    adapter: SystemAdapter,
}

impl WindowingSystem {
    pub(crate) fn new(
        component_definition: ComponentDefinition,
        config: WindowConfig,
    ) -> Result<Self> {
        let wayland_config = WaylandWindowConfig::from_domain_config(component_definition, config);
        let inner_system = WaylandWindowingSystem::new(wayland_config)?;
        let adapter = SystemAdapter::new(inner_system);

        Ok(Self { adapter })
    }

    #[must_use]
    pub fn event_loop_handle(&self) -> EventLoopHandle {
        EventLoopHandle {
            adapter: self.adapter.event_loop_handle(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.adapter.run()?;
        Ok(())
    }

    #[must_use]
    pub const fn component_instance(&self) -> &ComponentInstance {
        self.adapter.component_instance()
    }
}
