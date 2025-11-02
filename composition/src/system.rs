use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::{
    EventSource, Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer,
    channel,
};
use layer_shika_adapters::platform::slint_interpreter::{ComponentDefinition, ComponentInstance};
use layer_shika_adapters::wayland::{
    config::WaylandWindowConfig, shell_adapter::WaylandWindowingSystem,
    surfaces::surface_state::WindowState,
};
use layer_shika_domain::config::WindowConfig;
use std::cell::{Ref, RefCell};
use std::os::unix::io::AsFd;
use std::rc::{Rc, Weak};
use std::result::Result as StdResult;
use std::time::{Duration, Instant};

pub struct EventLoopHandle {
    system: Weak<RefCell<WaylandWindowingSystem>>,
}

impl EventLoopHandle {
    pub fn insert_source<S, F, R>(
        &self,
        source: S,
        mut callback: F,
    ) -> StdResult<RegistrationToken, Error>
    where
        S: EventSource<Ret = R> + 'static,
        F: FnMut(S::Event, &mut S::Metadata, RuntimeState<'_>) -> R + 'static,
    {
        let system = self.system.upgrade().ok_or(Error::SystemDropped)?;
        let loop_handle = system.borrow().event_loop_handle();

        loop_handle
            .insert_source(source, move |event, metadata, window_state| {
                let runtime_state = RuntimeState { window_state };
                callback(event, metadata, runtime_state)
            })
            .map_err(|e| {
                Error::Adapter(
                    EventLoopError::InsertSource {
                        message: format!("{e:?}"),
                    }
                    .into(),
                )
            })
    }

    pub fn add_timer<F>(&self, duration: Duration, mut callback: F) -> Result<RegistrationToken>
    where
        F: FnMut(Instant, RuntimeState<'_>) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_duration(duration);
        self.insert_source(timer, move |deadline, (), runtime_state| {
            callback(deadline, runtime_state)
        })
    }

    pub fn add_timer_at<F>(&self, deadline: Instant, mut callback: F) -> Result<RegistrationToken>
    where
        F: FnMut(Instant, RuntimeState<'_>) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_deadline(deadline);
        self.insert_source(timer, move |deadline, (), runtime_state| {
            callback(deadline, runtime_state)
        })
    }

    pub fn add_channel<T, F>(&self, mut callback: F) -> Result<(RegistrationToken, channel::Sender<T>)>
    where
        T: 'static,
        F: FnMut(T, RuntimeState<'_>) + 'static,
    {
        let (sender, receiver) = channel::channel();
        let token = self.insert_source(receiver, move |event, (), runtime_state| {
            if let channel::Event::Msg(msg) = event {
                callback(msg, runtime_state);
            }
        })?;
        Ok((token, sender))
    }

    pub fn add_fd<F, T>(&self, fd: T, interest: Interest, mode: Mode, mut callback: F) -> Result<RegistrationToken>
    where
        T: AsFd + 'static,
        F: FnMut(RuntimeState<'_>) + 'static,
    {
        let generic = Generic::new(fd, interest, mode);
        self.insert_source(generic, move |_readiness, _fd, runtime_state| {
            callback(runtime_state);
            Ok(PostAction::Continue)
        })
    }
}

pub struct RuntimeState<'a> {
    window_state: &'a mut WindowState,
}

impl RuntimeState<'_> {
    #[must_use]
    pub fn component_instance(&self) -> &ComponentInstance {
        self.window_state.component_instance()
    }

    pub fn render_frame_if_dirty(&mut self) -> Result<()> {
        Ok(self.window_state.render_frame_if_dirty()?)
    }
}

pub struct WindowingSystem {
    inner: Rc<RefCell<WaylandWindowingSystem>>,
}

impl WindowingSystem {
    pub(crate) fn new(
        component_definition: ComponentDefinition,
        config: WindowConfig,
    ) -> Result<Self> {
        let wayland_config = WaylandWindowConfig::from_domain_config(component_definition, config);
        let inner = WaylandWindowingSystem::new(wayland_config)?;

        Ok(Self {
            inner: Rc::new(RefCell::new(inner)),
        })
    }

    #[must_use]
    pub fn event_loop_handle(&self) -> EventLoopHandle {
        EventLoopHandle {
            system: Rc::downgrade(&self.inner),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.inner.borrow_mut().run()?;
        Ok(())
    }

    #[must_use]
    pub fn component_instance(&self) -> Ref<'_, ComponentInstance> {
        Ref::map(self.inner.borrow(), |system| system.component_instance())
    }
}
