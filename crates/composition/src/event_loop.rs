use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::{
    EventSource, Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer,
    channel,
};
use layer_shika_adapters::{AppState, WindowingSystemFacade};
use std::cell::RefCell;
use std::os::unix::io::AsFd;
use std::rc::Weak;
use std::time::{Duration, Instant};

pub trait FromAppState<'a> {
    fn from_app_state(app_state: &'a mut AppState) -> Self;
}

pub struct EventLoopHandleBase {
    system: Weak<RefCell<WindowingSystemFacade>>,
}

impl EventLoopHandleBase {
    pub fn new(system: Weak<RefCell<WindowingSystemFacade>>) -> Self {
        Self { system }
    }

    pub fn insert_source<S, F, R>(&self, source: S, callback: F) -> Result<RegistrationToken>
    where
        S: EventSource<Ret = R> + 'static,
        F: FnMut(S::Event, &mut S::Metadata, &mut AppState) -> R + 'static,
    {
        let system = self.system.upgrade().ok_or(Error::SystemDropped)?;
        let loop_handle = system.borrow().inner_ref().event_loop_handle();

        loop_handle.insert_source(source, callback).map_err(|e| {
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
        F: FnMut(Instant, &mut AppState) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_duration(duration);
        self.insert_source(timer, move |deadline, (), app_state| {
            callback(deadline, app_state)
        })
    }

    pub fn add_timer_at<F>(&self, deadline: Instant, mut callback: F) -> Result<RegistrationToken>
    where
        F: FnMut(Instant, &mut AppState) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_deadline(deadline);
        self.insert_source(timer, move |deadline, (), app_state| {
            callback(deadline, app_state)
        })
    }

    pub fn add_channel<T, F>(
        &self,
        mut callback: F,
    ) -> Result<(RegistrationToken, channel::Sender<T>)>
    where
        T: 'static,
        F: FnMut(T, &mut AppState) + 'static,
    {
        let (sender, receiver) = channel::channel();
        let token = self.insert_source(receiver, move |event, (), app_state| {
            if let channel::Event::Msg(msg) = event {
                callback(msg, app_state);
            }
        })?;
        Ok((token, sender))
    }

    pub fn add_fd<F, T>(
        &self,
        fd: T,
        interest: Interest,
        mode: Mode,
        mut callback: F,
    ) -> Result<RegistrationToken>
    where
        T: AsFd + 'static,
        F: FnMut(&mut AppState) + 'static,
    {
        let generic = Generic::new(fd, interest, mode);
        self.insert_source(generic, move |_readiness, _fd, app_state| {
            callback(app_state);
            Ok(PostAction::Continue)
        })
    }
}
