use crate::{Error, Result};
use layer_shika_adapters::errors::EventLoopError;
use layer_shika_adapters::platform::calloop::{
    EventSource, Generic, Interest, Mode, PostAction, RegistrationToken, TimeoutAction, Timer,
    channel,
};
use layer_shika_adapters::platform::slint::ComponentHandle;
use layer_shika_adapters::platform::slint_interpreter::{
    CompilationResult, ComponentDefinition, ComponentInstance, Value,
};
use layer_shika_adapters::wayland::{
    config::WaylandWindowConfig,
    shell_adapter::WaylandWindowingSystem,
    surfaces::{popup_manager::PopupManager, surface_state::WindowState},
};
use layer_shika_domain::config::WindowConfig;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
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

    pub fn add_channel<T, F>(
        &self,
        mut callback: F,
    ) -> Result<(RegistrationToken, channel::Sender<T>)>
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

    pub fn add_fd<F, T>(
        &self,
        fd: T,
        interest: Interest,
        mode: Mode,
        mut callback: F,
    ) -> Result<RegistrationToken>
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

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.window_state.compilation_result()
    }

    pub fn close_current_popup(&mut self) -> Result<()> {
        if let Some(popup_manager) = self.window_state.popup_manager() {
            popup_manager.close_current_popup();
        }
        Ok(())
    }

    fn measure_popup_dimensions(&mut self, definition: &ComponentDefinition) -> Result<(f32, f32)> {
        log::debug!(
            "Creating temporary popup instance to read dimensions from component properties"
        );

        let temp_instance = definition.create().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to create temporary popup instance: {}", e),
            })
        })?;

        temp_instance.show().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to show temporary popup instance: {}", e),
            })
        })?;

        let width: f32 = temp_instance
            .get_property("popup-width")
            .ok()
            .and_then(|v| v.try_into().ok())
            .unwrap_or(120.0);

        let height: f32 = temp_instance
            .get_property("popup-height")
            .ok()
            .and_then(|v| v.try_into().ok())
            .unwrap_or(120.0);

        log::debug!(
            "Read popup dimensions from component properties: {}x{} (popup-width, popup-height)",
            width,
            height
        );

        drop(temp_instance);
        self.close_current_popup()?;
        log::debug!("Destroyed temporary popup instance");

        Ok((width, height))
    }

    fn create_popup_instance(
        definition: &ComponentDefinition,
        popup_manager: &Rc<PopupManager>,
    ) -> Result<ComponentInstance> {
        let instance = definition.create().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to create popup instance: {}", e),
            })
        })?;

        let popup_manager_for_callback = Rc::clone(popup_manager);
        instance
            .set_callback("closed", move |_| {
                popup_manager_for_callback.close_current_popup();
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to set popup closed callback: {}", e),
                })
            })?;

        instance.show().map_err(|e| {
            Error::Domain(DomainError::Configuration {
                message: format!("Failed to show popup instance: {}", e),
            })
        })?;

        Ok(instance)
    }

    pub fn show_popup_component(
        &mut self,
        component_name: &str,
        position: Option<(f32, f32)>,
        size: Option<(f32, f32)>,
        positioning_mode: PopupPositioningMode,
    ) -> Result<()> {
        let compilation_result = self.compilation_result().ok_or_else(|| {
            Error::Domain(DomainError::Configuration {
                message: "No compilation result available for popup creation".to_string(),
            })
        })?;

        let definition = compilation_result
            .component(component_name)
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "{} component not found in compilation result",
                        component_name
                    ),
                })
            })?;

        self.close_current_popup()?;

        let (width, height) = if let Some(explicit_size) = size {
            log::debug!(
                "Using explicit popup size: {}x{}",
                explicit_size.0,
                explicit_size.1
            );
            explicit_size
        } else {
            self.measure_popup_dimensions(&definition)?
        };

        let popup_manager = self
            .window_state
            .popup_manager()
            .as_ref()
            .ok_or_else(|| {
                Error::Domain(DomainError::Configuration {
                    message: "No popup manager available".to_string(),
                })
            })
            .map(Rc::clone)?;

        let (reference_x, reference_y) = position.unwrap_or((0.0, 0.0));

        popup_manager.set_pending_popup_config(
            reference_x,
            reference_y,
            width,
            height,
            positioning_mode,
        );

        log::debug!(
            "Creating final popup instance with dimensions {}x{} at position ({}, {}), mode: {:?}",
            width,
            height,
            reference_x,
            reference_y,
            positioning_mode
        );

        Self::create_popup_instance(&definition, &popup_manager)?;

        Ok(())
    }
}

pub struct WindowingSystem {
    inner: Rc<RefCell<WaylandWindowingSystem>>,
    popup_positioning_mode: Rc<RefCell<PopupPositioningMode>>,
}

impl WindowingSystem {
    pub(crate) fn new(
        component_definition: ComponentDefinition,
        compilation_result: Option<Rc<CompilationResult>>,
        config: WindowConfig,
    ) -> Result<Self> {
        let wayland_config = WaylandWindowConfig::from_domain_config(
            component_definition,
            compilation_result,
            config,
        );
        let inner = WaylandWindowingSystem::new(wayland_config)?;

        let system = Self {
            inner: Rc::new(RefCell::new(inner)),
            popup_positioning_mode: Rc::new(RefCell::new(PopupPositioningMode::Center)),
        };

        system.register_popup_callbacks()?;

        Ok(system)
    }

    fn register_popup_callbacks(&self) -> Result<()> {
        let component_instance = self.component_instance();

        let popup_mode_clone = Rc::clone(&self.popup_positioning_mode);
        component_instance
            .set_callback("set_popup_positioning_mode", move |args| {
                let center_x: bool = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(false);
                let center_y: bool = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(false);

                let mode = PopupPositioningMode::from_flags(center_x, center_y);
                *popup_mode_clone.borrow_mut() = mode;
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!(
                        "Failed to register set_popup_positioning_mode callback: {}",
                        e
                    ),
                })
            })?;

        let event_loop_handle = self.event_loop_handle();
        let popup_mode_for_channel = Rc::clone(&self.popup_positioning_mode);

        let (_token, sender) = event_loop_handle.add_channel(
            move |(component_name, x, y): (String, f32, f32), mut state| {
                let mode = *popup_mode_for_channel.borrow();
                if let Err(e) =
                    state.show_popup_component(&component_name, Some((x, y)), None, mode)
                {
                    log::error!("Failed to show popup: {}", e);
                }
            },
        )?;

        component_instance
            .set_callback("show_popup", move |args| {
                use layer_shika_adapters::platform::slint::SharedString;

                let component_name: SharedString = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or_else(|| SharedString::from(""));

                if component_name.is_empty() {
                    log::error!("show_popup called without component name");
                    return Value::Void;
                }

                let x: f32 = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);
                let y: f32 = args
                    .get(2)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);

                if sender.send((component_name.to_string(), x, y)).is_err() {
                    log::error!("Failed to send popup request through channel");
                }
                Value::Void
            })
            .map_err(|e| {
                Error::Domain(DomainError::Configuration {
                    message: format!("Failed to register show_popup callback: {}", e),
                })
            })?;

        Ok(())
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
