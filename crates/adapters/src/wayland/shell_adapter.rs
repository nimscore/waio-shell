use crate::wayland::{
    config::{LayerSurfaceConfig, WaylandWindowConfig},
    globals::context::GlobalContext,
    surfaces::layer_surface::{SurfaceCtx, SurfaceSetupParams},
    surfaces::popup_manager::{PopupContext, PopupManager},
    surfaces::{
        event_context::SharedPointerSerial, surface_builder::WindowStateBuilder,
        surface_state::WindowState,
    },
};
use crate::{
    errors::{EventLoopError, LayerShikaError, RenderingError, Result},
    rendering::{
        egl::context::EGLContext,
        femtovg::{main_window::FemtoVGWindow, renderable_window::RenderableWindow},
        slint_integration::platform::CustomSlintPlatform,
    },
};
use core::result::Result as CoreResult;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::ports::windowing::WindowingSystemPort;
use log::{error, info};
use slint::{
    LogicalPosition, PhysicalSize, PlatformError, WindowPosition,
    platform::{WindowAdapter, femtovg_renderer::FemtoVGRenderer, update_timers_and_animations},
};
use slint_interpreter::ComponentInstance;
use smithay_client_toolkit::reexports::calloop::{
    EventLoop, Interest, LoopHandle, Mode, PostAction, generic::Generic,
};
use std::rc::Rc;
use wayland_client::{
    Connection, EventQueue, Proxy,
    protocol::{wl_display::WlDisplay, wl_surface::WlSurface},
};

pub struct WaylandWindowingSystem {
    state: WindowState,
    connection: Rc<Connection>,
    event_queue: EventQueue<WindowState>,
    event_loop: EventLoop<'static, WindowState>,
    popup_manager: Rc<PopupManager>,
}

impl WaylandWindowingSystem {
    pub fn new(config: WaylandWindowConfig) -> Result<Self> {
        info!("Initializing WindowingSystem");
        let (connection, event_queue) = Self::init_wayland_connection()?;
        let (state, global_ctx, platform) = Self::init_state(config, &connection, &event_queue)?;
        let event_loop =
            EventLoop::try_new().map_err(|e| EventLoopError::Creation { source: e })?;

        let popup_context = PopupContext::new(
            global_ctx.compositor,
            global_ctx.xdg_wm_base,
            global_ctx.seat,
            global_ctx.fractional_scale_manager,
            global_ctx.viewporter,
            connection.display(),
            Rc::clone(&connection),
        );

        let popup_manager = Rc::new(PopupManager::new(
            popup_context,
            Rc::clone(state.display_metrics()),
        ));
        let shared_serial = Rc::new(SharedPointerSerial::new());

        Self::setup_popup_creator(
            &popup_manager,
            &platform,
            &state,
            &event_queue,
            &shared_serial,
        );

        Ok(Self {
            state,
            connection,
            event_queue,
            event_loop,
            popup_manager,
        })
        .map(|mut system| {
            system
                .state
                .set_popup_manager(Rc::clone(&system.popup_manager));
            system.state.set_shared_pointer_serial(shared_serial);
            system
        })
    }

    fn init_wayland_connection() -> Result<(Rc<Connection>, EventQueue<WindowState>)> {
        let connection = Rc::new(Connection::connect_to_env()?);
        let event_queue = connection.new_event_queue();
        Ok((connection, event_queue))
    }

    fn init_state(
        config: WaylandWindowConfig,
        connection: &Connection,
        event_queue: &EventQueue<WindowState>,
    ) -> Result<(WindowState, GlobalContext, Rc<CustomSlintPlatform>)> {
        let global_ctx = GlobalContext::initialize(connection, &event_queue.handle())?;

        let layer_surface_config = LayerSurfaceConfig {
            anchor: config.anchor,
            margin: config.margin,
            exclusive_zone: config.exclusive_zone,
            keyboard_interactivity: config.keyboard_interactivity,
            height: config.height,
        };

        let setup_params = SurfaceSetupParams {
            compositor: &global_ctx.compositor,
            output: &global_ctx.output,
            layer_shell: &global_ctx.layer_shell,
            fractional_scale_manager: global_ctx.fractional_scale_manager.as_ref(),
            viewporter: global_ctx.viewporter.as_ref(),
            queue_handle: &event_queue.handle(),
            layer: config.layer,
            namespace: config.namespace.clone(),
        };

        let surface_ctx = SurfaceCtx::setup(&setup_params, &layer_surface_config);

        let window =
            Self::initialize_renderer(&surface_ctx.surface, &connection.display(), &config)?;

        let pointer = Rc::new(global_ctx.seat.get_pointer(&event_queue.handle(), ()));

        let mut builder = WindowStateBuilder::new()
            .with_component_definition(config.component_definition)
            .with_compilation_result(config.compilation_result)
            .with_surface(Rc::clone(&surface_ctx.surface))
            .with_layer_surface(Rc::clone(&surface_ctx.layer_surface))
            .with_scale_factor(config.scale_factor)
            .with_height(config.height)
            .with_exclusive_zone(config.exclusive_zone)
            .with_connection(Rc::new(connection.clone()))
            .with_pointer(Rc::clone(&pointer))
            .with_window(window);

        if let Some(fs) = &surface_ctx.fractional_scale {
            builder = builder.with_fractional_scale(Rc::clone(fs));
        }

        if let Some(vp) = &surface_ctx.viewport {
            builder = builder.with_viewport(Rc::clone(vp));
        }

        let (state, platform) =
            builder
                .build()
                .map_err(|e| LayerShikaError::WindowConfiguration {
                    message: e.to_string(),
                })?;

        Ok((state, global_ctx, platform))
    }

    fn setup_popup_creator(
        popup_manager: &Rc<PopupManager>,
        platform: &Rc<CustomSlintPlatform>,
        state: &WindowState,
        event_queue: &EventQueue<WindowState>,
        shared_serial: &Rc<SharedPointerSerial>,
    ) {
        if !popup_manager.has_xdg_shell() {
            info!("xdg-shell not available, popups will not be supported");
            return;
        }

        info!("Setting up popup creator with xdg-shell support");

        let popup_manager_clone = Rc::clone(popup_manager);
        let layer_surface = state.layer_surface();
        let queue_handle = event_queue.handle();
        let serial_holder = Rc::clone(shared_serial);

        platform.set_popup_creator(move || {
            info!("Popup creator called! Creating popup window...");

            let serial = serial_holder.get();

            let popup_window = popup_manager_clone
                .create_pending_popup(&queue_handle, &layer_surface, serial)
                .map_err(|e| PlatformError::Other(format!("Failed to create popup: {e}")))?;

            let result = Ok(popup_window as Rc<dyn WindowAdapter>);

            match &result {
                Ok(_) => info!("Popup created successfully"),
                Err(e) => info!("Popup creation failed: {e:?}"),
            }

            result
        });
    }

    fn initialize_renderer(
        surface: &Rc<WlSurface>,
        display: &WlDisplay,
        config: &WaylandWindowConfig,
    ) -> Result<Rc<FemtoVGWindow>> {
        let init_size = PhysicalSize::new(1, 1);

        let context = EGLContext::builder()
            .with_display_id(display.id())
            .with_surface_id(surface.id())
            .with_size(init_size)
            .build()?;

        let renderer = FemtoVGRenderer::new(context)
            .map_err(|e| LayerShikaError::FemtoVGRendererCreation { source: e })?;

        let femtovg_window = FemtoVGWindow::new(renderer);
        femtovg_window.set_size(slint::WindowSize::Physical(init_size));
        femtovg_window.set_scale_factor(config.scale_factor);
        femtovg_window.set_position(WindowPosition::Logical(LogicalPosition::new(0., 0.)));

        Ok(femtovg_window)
    }

    pub fn event_loop_handle(&self) -> LoopHandle<'static, WindowState> {
        self.event_loop.handle()
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Starting WindowingSystem main loop");

        info!("Processing initial Wayland configuration events");
        while self.event_queue.blocking_dispatch(&mut self.state)? > 0 {
            self.connection
                .flush()
                .map_err(|e| LayerShikaError::WaylandProtocol { source: e })?;

            update_timers_and_animations();
            self.state
                .window()
                .render_frame_if_dirty()
                .map_err(|e| RenderingError::Operation {
                    message: e.to_string(),
                })?;
        }

        self.setup_wayland_event_source()?;

        let event_queue = &mut self.event_queue;
        let connection = &self.connection;
        let popup_manager = Rc::clone(&self.popup_manager);

        self.event_loop
            .run(None, &mut self.state, move |shared_data| {
                if let Err(e) =
                    Self::process_events(connection, event_queue, shared_data, &popup_manager)
                {
                    error!("Error processing events: {e}");
                }
            })
            .map_err(|e| EventLoopError::Execution { source: e })?;

        Ok(())
    }

    fn setup_wayland_event_source(&self) -> Result<()> {
        let connection = Rc::clone(&self.connection);

        self.event_loop
            .handle()
            .insert_source(
                Generic::new(connection, Interest::READ, Mode::Level),
                move |_, _connection, _shared_data| Ok(PostAction::Continue),
            )
            .map_err(|e| EventLoopError::InsertSource {
                message: format!("{e:?}"),
            })?;

        Ok(())
    }

    fn process_events(
        connection: &Connection,
        event_queue: &mut EventQueue<WindowState>,
        shared_data: &mut WindowState,
        popup_manager: &PopupManager,
    ) -> Result<()> {
        if let Some(guard) = event_queue.prepare_read() {
            guard
                .read()
                .map_err(|e| LayerShikaError::WaylandProtocol { source: e })?;
        }

        event_queue.dispatch_pending(shared_data)?;

        update_timers_and_animations();

        shared_data
            .window()
            .render_frame_if_dirty()
            .map_err(|e| RenderingError::Operation {
                message: e.to_string(),
            })?;

        popup_manager
            .render_popups()
            .map_err(|e| RenderingError::Operation {
                message: e.to_string(),
            })?;

        connection
            .flush()
            .map_err(|e| LayerShikaError::WaylandProtocol { source: e })?;

        Ok(())
    }

    pub const fn component_instance(&self) -> &ComponentInstance {
        self.state.component_instance()
    }

    pub const fn state(&self) -> &WindowState {
        &self.state
    }
}

impl WindowingSystemPort for WaylandWindowingSystem {
    fn run(&mut self) -> CoreResult<(), DomainError> {
        WaylandWindowingSystem::run(self).map_err(|e| DomainError::Adapter {
            source: Box::new(e),
        })
    }
}
