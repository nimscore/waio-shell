use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::{
    EventSource, Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer,
    channel,
};
use layer_shika_adapters::{AppState, WindowingSystemFacade};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::os::unix::io::AsFd;
use std::rc::Weak;
use std::time::{Duration, Instant};

pub trait FromAppState<'a> {
    fn from_app_state(app_state: &'a mut AppState) -> Self;
}

pub struct EventLoopHandleBase<Ctx> {
    system: Weak<RefCell<WindowingSystemFacade>>,
    _marker: PhantomData<fn(&mut AppState) -> Ctx>,
}

impl<Ctx> EventLoopHandleBase<Ctx> {
    pub fn new(system: Weak<RefCell<WindowingSystemFacade>>) -> Self {
        Self {
            system,
            _marker: PhantomData,
        }
    }
}

impl<Ctx> EventLoopHandleBase<Ctx>
where
    for<'a> Ctx: FromAppState<'a> + 'a,
{
    pub fn insert_source<S, F, R>(&self, source: S, mut callback: F) -> Result<RegistrationToken>
    where
        S: EventSource<Ret = R> + 'static,
        F: FnMut(S::Event, &mut S::Metadata, Ctx) -> R + 'static,
    {
        let system = self.system.upgrade().ok_or(Error::SystemDropped)?;
        let loop_handle = system.borrow().inner_ref().event_loop_handle();

        loop_handle
            .insert_source(source, move |event, metadata, app_state| {
                let ctx = Ctx::from_app_state(app_state);
                callback(event, metadata, ctx)
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
        F: FnMut(Instant, Ctx) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_duration(duration);
        self.insert_source(timer, move |deadline, (), ctx| callback(deadline, ctx))
    }

    pub fn add_timer_at<F>(&self, deadline: Instant, mut callback: F) -> Result<RegistrationToken>
    where
        F: FnMut(Instant, Ctx) -> TimeoutAction + 'static,
    {
        let timer = Timer::from_deadline(deadline);
        self.insert_source(timer, move |deadline, (), ctx| callback(deadline, ctx))
    }

    pub fn add_channel<T, F>(
        &self,
        mut callback: F,
    ) -> Result<(RegistrationToken, channel::Sender<T>)>
    where
        T: 'static,
        F: FnMut(T, Ctx) + 'static,
    {
        let (sender, receiver) = channel::channel();
        let token = self.insert_source(receiver, move |event, (), ctx| {
            if let channel::Event::Msg(msg) = event {
                callback(msg, ctx);
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
        F: FnMut(Ctx) + 'static,
    {
        let generic = Generic::new(fd, interest, mode);
        self.insert_source(generic, move |_readiness, _fd, ctx| {
            callback(ctx);
            Ok(PostAction::Continue)
        })
    }
}
