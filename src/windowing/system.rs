use super::{
    config::{LayerSurfaceParams, WindowConfig},
    globals::GlobalCtx,
    popup_manager::{PopupContext, PopupManager},
    state::{builder::WindowStateBuilder, WindowState},
    surface::{SurfaceCtx, SurfaceSetupParams},
};
use crate::{
    errors::{LayerShikaError, Result},
    rendering::{
        egl_context::EGLContext, femtovg_window::FemtoVGWindow, slint_platform::CustomSlintPlatform,
    },
};
use log::{error, info};
use slint::{
    platform::{femtovg_renderer::FemtoVGRenderer, update_timers_and_animations, WindowAdapter},
    LogicalPosition, PhysicalSize, PlatformError, WindowPosition,
};
use slint_interpreter::ComponentInstance;
use smithay_client_toolkit::reexports::calloop::{
    generic::Generic, EventLoop, Interest, LoopHandle, Mode, PostAction,
};
use std::rc::Rc;
use wayland_client::{
    protocol::{wl_display::WlDisplay, wl_surface::WlSurface},
    Connection, EventQueue, Proxy,
};

pub struct WindowingSystem {
    state: WindowState,
    connection: Rc<Connection>,
    event_queue: EventQueue<WindowState>,
    event_loop: EventLoop<'static, WindowState>,
    popup_manager: Rc<PopupManager>,
}

impl WindowingSystem {
    pub(super) fn new(config: WindowConfig) -> Result<Self> {
        info!("Initializing WindowingSystem");
        let (connection, event_queue) = Self::init_wayland_connection()?;
        let (state, global_ctx, platform) = Self::init_state(config, &connection, &event_queue)?;
        let event_loop =
            EventLoop::try_new().map_err(|e| LayerShikaError::EventLoop(e.to_string()))?;

        let popup_context = PopupContext::new(
            global_ctx.compositor,
            global_ctx.xdg_wm_base,
            global_ctx.seat,
            global_ctx.fractional_scale_manager,
            global_ctx.viewporter,
            connection.display(),
            Rc::clone(&connection),
        );

        let popup_manager = Rc::new(PopupManager::new(popup_context, state.scale_factor()));

        Self::setup_popup_creator(&popup_manager, &platform, &state, &event_queue);

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
            system
        })
    }

    fn init_wayland_connection() -> Result<(Rc<Connection>, EventQueue<WindowState>)> {
        let connection =
            Rc::new(Connection::connect_to_env().map_err(LayerShikaError::WaylandConnection)?);
        let event_queue = connection.new_event_queue();
        Ok((connection, event_queue))
    }

    fn init_state(
        config: WindowConfig,
        connection: &Connection,
        event_queue: &EventQueue<WindowState>,
    ) -> Result<(WindowState, GlobalCtx, Rc<CustomSlintPlatform>)> {
        let global_ctx = GlobalCtx::initialize(connection, &event_queue.handle())
            .map_err(|e| LayerShikaError::GlobalInitialization(e.to_string()))?;

        let layer_surface_params = LayerSurfaceParams {
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

        let surface_ctx = SurfaceCtx::setup(&setup_params, &layer_surface_params);

        let pointer = Rc::new(global_ctx.seat.get_pointer(&event_queue.handle(), ()));
        let output = Rc::new(global_ctx.output.clone());
        let window =
            Self::initialize_renderer(&surface_ctx.surface, &connection.display(), &config)
                .map_err(|e| LayerShikaError::EGLContextCreation(e.to_string()))?;

        let mut builder = WindowStateBuilder::new()
            .with_component_definition(config.component_definition)
            .with_surface(Rc::clone(&surface_ctx.surface))
            .with_layer_surface(Rc::clone(&surface_ctx.layer_surface))
            .with_pointer(Rc::clone(&pointer))
            .with_output(Rc::clone(&output))
            .with_scale_factor(config.scale_factor)
            .with_height(config.height)
            .with_exclusive_zone(config.exclusive_zone)
            .with_connection(Rc::new(connection.clone()))
            .with_window(window);

        if let Some(fs) = &surface_ctx.fractional_scale {
            builder = builder.with_fractional_scale(Rc::clone(fs));
        }

        if let Some(vp) = &surface_ctx.viewport {
            builder = builder.with_viewport(Rc::clone(vp));
        }

        let (state, platform) = builder
            .build()
            .map_err(|e| LayerShikaError::WindowConfiguration(e.to_string()))?;

        Ok((state, global_ctx, platform))
    }

    fn setup_popup_creator(
        popup_manager: &Rc<PopupManager>,
        platform: &Rc<CustomSlintPlatform>,
        state: &WindowState,
        event_queue: &EventQueue<WindowState>,
    ) {
        if !popup_manager.has_xdg_shell() {
            info!("xdg-shell not available, popups will not be supported");
            return;
        }

        info!("Setting up popup creator with xdg-shell support");

        let popup_manager_clone = Rc::clone(popup_manager);
        let layer_surface = state.layer_surface();
        let queue_handle = event_queue.handle();

        platform.set_popup_creator(move || {
            info!("Popup creator called! Creating popup window...");

            let result = popup_manager_clone
                .create_popup(&queue_handle, &layer_surface, 0)
                .map(|w| w as Rc<dyn WindowAdapter>)
                .map_err(|e| PlatformError::Other(format!("Failed to create popup: {e}")));

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
        config: &WindowConfig,
    ) -> Result<Rc<FemtoVGWindow>> {
        let init_size = PhysicalSize::new(1, 1);

        let context = EGLContext::builder()
            .with_display_id(display.id())
            .with_surface_id(surface.id())
            .with_size(init_size)
            .build()
            .map_err(|e| LayerShikaError::EGLContextCreation(e.to_string()))?;

        let renderer = FemtoVGRenderer::new(context)
            .map_err(|e| LayerShikaError::FemtoVGRendererCreation(e.to_string()))?;

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
        while self
            .event_queue
            .blocking_dispatch(&mut self.state)
            .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?
            > 0
        {
            self.connection
                .flush()
                .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?;

            update_timers_and_animations();
            self.state
                .window()
                .render_frame_if_dirty()
                .map_err(|e| LayerShikaError::Rendering(e.to_string()))?;
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
            .map_err(|e| LayerShikaError::EventLoop(e.to_string()))
    }

    fn setup_wayland_event_source(&self) -> Result<()> {
        let connection = Rc::clone(&self.connection);

        self.event_loop
            .handle()
            .insert_source(
                Generic::new(connection, Interest::READ, Mode::Level),
                move |_, _connection, _shared_data| Ok(PostAction::Continue),
            )
            .map_err(|e| LayerShikaError::EventLoop(e.to_string()))?;

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
                .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?;
        }

        event_queue
            .dispatch_pending(shared_data)
            .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?;

        update_timers_and_animations();

        shared_data
            .window()
            .render_frame_if_dirty()
            .map_err(|e| LayerShikaError::Rendering(e.to_string()))?;

        popup_manager
            .render_popups()
            .map_err(|e| LayerShikaError::Rendering(e.to_string()))?;

        connection
            .flush()
            .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?;

        Ok(())
    }

    pub const fn component_instance(&self) -> &ComponentInstance {
        self.state.component_instance()
    }

    pub fn window(&self) -> Rc<FemtoVGWindow> {
        self.state.window()
    }

    pub const fn state(&self) -> &WindowState {
        &self.state
    }
}
