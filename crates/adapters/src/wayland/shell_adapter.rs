use crate::wayland::{
    config::{LayerSurfaceConfig, WaylandWindowConfig},
    globals::context::GlobalContext,
    managed_proxies::ManagedWlPointer,
    surfaces::layer_surface::{SurfaceCtx, SurfaceSetupParams},
    surfaces::popup_manager::{PopupContext, PopupManager},
    surfaces::{
        app_state::AppState,
        event_context::SharedPointerSerial,
        surface_builder::{PlatformWrapper, WindowStateBuilder},
        surface_state::WindowState,
    },
};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use crate::{
    errors::{EventLoopError, LayerShikaError, RenderingError, Result},
    rendering::{
        egl::context_factory::RenderContextFactory,
        femtovg::{main_window::FemtoVGWindow, renderable_window::RenderableWindow},
        slint_integration::platform::CustomSlintPlatform,
    },
};
use core::result::Result as CoreResult;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::ports::windowing::WindowingSystemPort;
use layer_shika_domain::value_objects::output_handle::OutputHandle;
use layer_shika_domain::value_objects::output_info::OutputInfo;
use log::{error, info};
use slint::{
    LogicalPosition, PhysicalSize, PlatformError, WindowPosition,
    platform::{WindowAdapter, femtovg_renderer::FemtoVGRenderer, set_platform, update_timers_and_animations},
};
use slint_interpreter::ComponentInstance;
use smithay_client_toolkit::reexports::calloop::{
    EventLoop, Interest, LoopHandle, Mode, PostAction, generic::Generic,
};
use std::rc::Rc;
use wayland_client::{
    Connection, EventQueue, Proxy, QueueHandle,
    backend::ObjectId,
    protocol::{wl_pointer::WlPointer, wl_surface::WlSurface},
};

type PopupManagersAndSurfaces = (Vec<Rc<PopupManager>>, Vec<Rc<ZwlrLayerSurfaceV1>>);

struct OutputSetup {
    output_id: ObjectId,
    main_surface_id: ObjectId,
    window: Rc<FemtoVGWindow>,
    builder: WindowStateBuilder,
}

pub struct WaylandWindowingSystem {
    state: AppState,
    connection: Rc<Connection>,
    event_queue: EventQueue<AppState>,
    event_loop: EventLoop<'static, AppState>,
}

impl WaylandWindowingSystem {
    pub fn new(config: &WaylandWindowConfig) -> Result<Self> {
        info!("Initializing WindowingSystem");
        let (connection, mut event_queue) = Self::init_wayland_connection()?;
        let event_loop =
            EventLoop::try_new().map_err(|e| EventLoopError::Creation { source: e })?;

        let state = Self::init_state(config, &connection, &mut event_queue)?;

        Ok(Self {
            state,
            connection,
            event_queue,
            event_loop,
        })
    }

    fn init_wayland_connection() -> Result<(Rc<Connection>, EventQueue<AppState>)> {
        let connection = Rc::new(Connection::connect_to_env()?);
        let event_queue = connection.new_event_queue();
        Ok((connection, event_queue))
    }

    fn create_layer_surface_config(config: &WaylandWindowConfig) -> LayerSurfaceConfig {
        LayerSurfaceConfig {
            anchor: config.anchor,
            margin: config.margin,
            exclusive_zone: config.exclusive_zone,
            keyboard_interactivity: config.keyboard_interactivity,
            height: config.height,
        }
    }

    fn create_output_setups(
        config: &WaylandWindowConfig,
        global_ctx: &GlobalContext,
        connection: &Connection,
        event_queue: &mut EventQueue<AppState>,
        pointer: &Rc<WlPointer>,
        layer_surface_config: &LayerSurfaceConfig,
    ) -> Result<Vec<OutputSetup>> {
        let mut setups = Vec::new();

        for (index, output) in global_ctx.outputs.iter().enumerate() {
            let is_primary = index == 0;

            let mut temp_info = OutputInfo::new(OutputHandle::new());
            temp_info.set_primary(is_primary);

            if !config.output_policy.should_render(&temp_info) {
                info!("Skipping output {} due to output policy", index);
                continue;
            }

            let output_id = output.id();

            let setup_params = SurfaceSetupParams {
                compositor: &global_ctx.compositor,
                output,
                layer_shell: &global_ctx.layer_shell,
                fractional_scale_manager: global_ctx.fractional_scale_manager.as_ref(),
                viewporter: global_ctx.viewporter.as_ref(),
                queue_handle: &event_queue.handle(),
                layer: config.layer,
                namespace: config.namespace.clone(),
            };

            let surface_ctx = SurfaceCtx::setup(&setup_params, layer_surface_config);
            let main_surface_id = surface_ctx.surface.id();

            let render_factory =
                RenderContextFactory::new(Rc::clone(&global_ctx.render_context_manager));

            let window = Self::initialize_renderer(&surface_ctx.surface, config, &render_factory)?;

            let mut builder = WindowStateBuilder::new()
                .with_component_definition(config.component_definition.clone())
                .with_compilation_result(config.compilation_result.clone())
                .with_surface(Rc::clone(&surface_ctx.surface))
                .with_layer_surface(Rc::clone(&surface_ctx.layer_surface))
                .with_scale_factor(config.scale_factor)
                .with_height(config.height)
                .with_exclusive_zone(config.exclusive_zone)
                .with_connection(Rc::new(connection.clone()))
                .with_pointer(Rc::clone(pointer))
                .with_window(Rc::clone(&window));

            if let Some(fs) = &surface_ctx.fractional_scale {
                builder = builder.with_fractional_scale(Rc::clone(fs));
            }

            if let Some(vp) = &surface_ctx.viewport {
                builder = builder.with_viewport(Rc::clone(vp));
            }

            setups.push(OutputSetup {
                output_id,
                main_surface_id,
                window,
                builder,
            });
        }

        Ok(setups)
    }

    fn setup_platform(setups: &[OutputSetup]) -> Result<Rc<CustomSlintPlatform>> {
        let first_setup = setups
            .first()
            .ok_or_else(|| LayerShikaError::InvalidInput {
                message: "No outputs available".into(),
            })?;

        let platform = CustomSlintPlatform::new(&first_setup.window);

        for setup in setups.iter().skip(1) {
            platform.add_window(Rc::clone(&setup.window));
        }

        set_platform(Box::new(PlatformWrapper(Rc::clone(&platform))))
            .map_err(|e| LayerShikaError::PlatformSetup { source: e })?;

        Ok(platform)
    }

    fn create_window_states(
        setups: Vec<OutputSetup>,
        popup_context: &PopupContext,
        shared_serial: &Rc<SharedPointerSerial>,
        app_state: &mut AppState,
    ) -> Result<PopupManagersAndSurfaces> {
        let mut popup_managers = Vec::new();
        let mut layer_surfaces = Vec::new();

        for setup in setups {
            let mut per_output_window = WindowState::new(setup.builder).map_err(|e| {
                LayerShikaError::WindowConfiguration {
                    message: e.to_string(),
                }
            })?;

            let popup_manager = Rc::new(PopupManager::new(
                popup_context.clone(),
                Rc::clone(per_output_window.display_metrics()),
            ));

            per_output_window.set_popup_manager(Rc::clone(&popup_manager));
            per_output_window.set_shared_pointer_serial(Rc::clone(shared_serial));

            popup_managers.push(Rc::clone(&popup_manager));
            layer_surfaces.push(per_output_window.layer_surface());

            app_state.add_output(setup.output_id, setup.main_surface_id, per_output_window);
        }

        Ok((popup_managers, layer_surfaces))
    }

    fn init_state(
        config: &WaylandWindowConfig,
        connection: &Connection,
        event_queue: &mut EventQueue<AppState>,
    ) -> Result<AppState> {
        let global_ctx = GlobalContext::initialize(connection, &event_queue.handle())?;
        let layer_surface_config = Self::create_layer_surface_config(config);

        let pointer = Rc::new(global_ctx.seat.get_pointer(&event_queue.handle(), ()));
        let shared_serial = Rc::new(SharedPointerSerial::new());

        let mut app_state = AppState::new(
            ManagedWlPointer::new(Rc::clone(&pointer), Rc::new(connection.clone())),
            Rc::clone(&shared_serial),
        );

        let render_factory =
            RenderContextFactory::new(Rc::clone(&global_ctx.render_context_manager));

        let popup_context = PopupContext::new(
            global_ctx.compositor.clone(),
            global_ctx.xdg_wm_base.clone(),
            global_ctx.seat.clone(),
            global_ctx.fractional_scale_manager.clone(),
            global_ctx.viewporter.clone(),
            Rc::new(connection.clone()),
            Rc::clone(&render_factory),
        );

        let setups = Self::create_output_setups(
            config,
            &global_ctx,
            connection,
            event_queue,
            &pointer,
            &layer_surface_config,
        )?;

        let platform = Self::setup_platform(&setups)?;

        let (popup_managers, layer_surfaces) =
            Self::create_window_states(setups, &popup_context, &shared_serial, &mut app_state)?;

        Self::setup_shared_popup_creator(
            popup_managers,
            layer_surfaces,
            &platform,
            &event_queue.handle(),
            &shared_serial,
        );

        Ok(app_state)
    }

    fn setup_shared_popup_creator(
        popup_managers: Vec<Rc<PopupManager>>,
        layer_surfaces: Vec<Rc<ZwlrLayerSurfaceV1>>,
        platform: &Rc<CustomSlintPlatform>,
        queue_handle: &QueueHandle<AppState>,
        shared_serial: &Rc<SharedPointerSerial>,
    ) {
        let Some(first_manager) = popup_managers.first() else {
            info!("No popup managers available");
            return;
        };

        if !first_manager.has_xdg_shell() {
            info!("xdg-shell not available, popups will not be supported");
            return;
        }

        info!(
            "Setting up shared popup creator for {} output(s)",
            popup_managers.len()
        );

        let queue_handle_clone = queue_handle.clone();
        let serial_holder = Rc::clone(shared_serial);

        platform.set_popup_creator(move || {
            info!("Popup creator called! Searching for pending popup...");

            let serial = serial_holder.get();

            for (idx, (popup_manager, layer_surface)) in
                popup_managers.iter().zip(layer_surfaces.iter()).enumerate()
            {
                if popup_manager.has_pending_popup() {
                    info!("Found pending popup in output #{}", idx);

                    let popup_window = popup_manager
                        .create_pending_popup(&queue_handle_clone, layer_surface, serial)
                        .map_err(|e| {
                            PlatformError::Other(format!("Failed to create popup: {e}"))
                        })?;

                    info!("Popup created successfully for output #{}", idx);
                    return Ok(popup_window as Rc<dyn WindowAdapter>);
                }
            }

            Err(PlatformError::Other(
                "No pending popup request found in any output".into(),
            ))
        });
    }

    fn initialize_renderer(
        surface: &Rc<WlSurface>,
        config: &WaylandWindowConfig,
        render_factory: &Rc<RenderContextFactory>,
    ) -> Result<Rc<FemtoVGWindow>> {
        let init_size = PhysicalSize::new(1, 1);

        let context = render_factory.create_context(&surface.id(), init_size)?;

        let renderer = FemtoVGRenderer::new(context)
            .map_err(|e| LayerShikaError::FemtoVGRendererCreation { source: e })?;

        let femtovg_window = FemtoVGWindow::new(renderer);
        femtovg_window.set_size(slint::WindowSize::Physical(init_size));
        femtovg_window.set_scale_factor(config.scale_factor);
        femtovg_window.set_position(WindowPosition::Logical(LogicalPosition::new(0., 0.)));

        Ok(femtovg_window)
    }

    pub fn event_loop_handle(&self) -> LoopHandle<'static, AppState> {
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

            for window in self.state.all_outputs() {
                window
                    .window()
                    .render_frame_if_dirty()
                    .map_err(|e| RenderingError::Operation {
                        message: e.to_string(),
                    })?;
            }
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
        event_queue: &mut EventQueue<AppState>,
        shared_data: &mut AppState,
    ) -> Result<()> {
        if let Some(guard) = event_queue.prepare_read() {
            guard
                .read()
                .map_err(|e| LayerShikaError::WaylandProtocol { source: e })?;
        }

        event_queue.dispatch_pending(shared_data)?;

        update_timers_and_animations();

        for window in shared_data.all_outputs() {
            window
                .window()
                .render_frame_if_dirty()
                .map_err(|e| RenderingError::Operation {
                    message: e.to_string(),
                })?;

            if let Some(popup_manager) = window.popup_manager() {
                popup_manager
                    .render_popups()
                    .map_err(|e| RenderingError::Operation {
                        message: e.to_string(),
                    })?;
            }
        }

        connection
            .flush()
            .map_err(|e| LayerShikaError::WaylandProtocol { source: e })?;

        Ok(())
    }

    pub fn component_instance(&self) -> Result<&ComponentInstance> {
        self.state
            .primary_output()
            .ok_or_else(|| LayerShikaError::InvalidInput {
                message: "No outputs available".into(),
            })
            .map(WindowState::component_instance)
    }

    pub fn state(&self) -> Result<&WindowState> {
        self.state
            .primary_output()
            .ok_or_else(|| LayerShikaError::InvalidInput {
                message: "No outputs available".into(),
            })
    }

    pub fn app_state(&self) -> &AppState {
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
