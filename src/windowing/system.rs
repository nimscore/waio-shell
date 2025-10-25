use super::{
    config::{LayerSurfaceParams, WindowConfig},
    globals::GlobalCtx,
    state::{builder::WindowStateBuilder, WindowState},
    surface::{SurfaceCtx, SurfaceSetupParams},
};
use crate::{
    errors::{LayerShikaError, Result},
    rendering::{egl_context::EGLContext, femtovg_window::FemtoVGWindow},
};
use log::{error, info};
use slint::{
    platform::{femtovg_renderer::FemtoVGRenderer, update_timers_and_animations},
    LogicalPosition, PhysicalSize,
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
}

impl WindowingSystem {
    pub(super) fn new(config: WindowConfig) -> Result<Self> {
        info!("Initializing WindowingSystem");
        let connection =
            Rc::new(Connection::connect_to_env().map_err(LayerShikaError::WaylandConnection)?);
        let event_queue = connection.new_event_queue();

        let global_ctx = GlobalCtx::initialize(&connection, &event_queue.handle())
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
        let output = Rc::new(global_ctx.output);
        let window = Self::initialize_renderer(&surface_ctx.surface, &connection.display(), &config)
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
            .with_window(window);

        if let Some(fs) = &surface_ctx.fractional_scale {
            builder = builder.with_fractional_scale(Rc::clone(fs));
        }

        if let Some(vp) = &surface_ctx.viewport {
            builder = builder.with_viewport(Rc::clone(vp));
        }

        let state = builder
            .build()
            .map_err(|e| LayerShikaError::WindowConfiguration(e.to_string()))?;

        let event_loop =
            EventLoop::try_new().map_err(|e| LayerShikaError::EventLoop(e.to_string()))?;

        Ok(Self {
            state,
            connection,
            event_queue,
            event_loop,
        })
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
        femtovg_window.set_position(LogicalPosition::new(0., 0.));

        Ok(femtovg_window)
    }

    pub fn event_loop_handle(&self) -> LoopHandle<'static, WindowState> {
        self.event_loop.handle()
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Starting WindowingSystem main loop");

        while self
            .event_queue
            .blocking_dispatch(&mut self.state)
            .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?
            > 0
        {
            self.connection
                .flush()
                .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?;
            self.state
                .window()
                .render_frame_if_dirty()
                .map_err(|e| LayerShikaError::Rendering(e.to_string()))?;
        }

        self.setup_wayland_event_source()?;

        let event_queue = &mut self.event_queue;
        let connection = &self.connection;

        self.event_loop
            .run(None, &mut self.state, move |shared_data| {
                if let Err(e) = Self::process_events(connection, event_queue, shared_data) {
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
    ) -> Result<()> {
        if let Some(guard) = event_queue.prepare_read() {
            guard
                .read()
                .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?;
        }
        connection.flush()?;

        event_queue
            .dispatch_pending(shared_data)
            .map_err(|e| LayerShikaError::WaylandProtocol(e.to_string()))?;

        update_timers_and_animations();

        shared_data
            .window()
            .render_frame_if_dirty()
            .map_err(|e| LayerShikaError::Rendering(e.to_string()))?;

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
