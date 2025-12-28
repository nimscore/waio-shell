use crate::errors::{LayerShikaError, Result};
use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::rendering::femtovg::renderable_window::RenderableWindow;
use crate::rendering::slint_integration::platform::CustomSlintPlatform;
use crate::wayland::session_lock::lock_context::{LockSurfaceParams, SessionLockContext};
use crate::wayland::session_lock::lock_surface::LockSurface;
use crate::wayland::surfaces::app_state::AppState;
use crate::wayland::surfaces::component_state::ComponentState;
use crate::wayland::surfaces::display_metrics::DisplayMetrics;
use crate::wayland::surfaces::keyboard_state::{KeyboardState, keysym_to_text};
use crate::wayland::surfaces::pointer_utils::wayland_button_to_slint;
use layer_shika_domain::surface_dimensions::SurfaceDimensions;
use layer_shika_domain::value_objects::lock_config::LockConfig;
use layer_shika_domain::value_objects::lock_state::LockState;
use log::info;
use slint::{
    LogicalPosition, LogicalSize, SharedString, WindowPosition, WindowSize,
    platform::{WindowAdapter, WindowEvent, femtovg_renderer::FemtoVGRenderer},
};
use slint_interpreter::{CompilationResult, ComponentDefinition, ComponentInstance, Value};
use std::collections::HashMap;
use std::rc::Rc;
use wayland_client::{
    Proxy, QueueHandle, WEnum,
    backend::ObjectId,
    protocol::{wl_keyboard, wl_output::WlOutput, wl_pointer, wl_surface::WlSurface},
};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1::ExtSessionLockV1;
use xkbcommon::xkb;

type LockCallbackHandler = Rc<dyn Fn(&[Value]) -> Value>;

#[derive(Clone)]
pub(crate) struct LockCallback {
    name: String,
    handler: LockCallbackHandler,
}

impl LockCallback {
    pub fn new(name: impl Into<String>, handler: LockCallbackHandler) -> Self {
        Self {
            name: name.into(),
            handler,
        }
    }

    pub fn apply_to(&self, component: &ComponentInstance) -> Result<()> {
        let handler = Rc::clone(&self.handler);
        component
            .set_callback(&self.name, move |args| handler(args))
            .map_err(|e| LayerShikaError::InvalidInput {
                message: format!("Failed to register callback '{}': {e}", self.name),
            })
    }
}

struct ActiveLockSurface {
    surface: LockSurface,
    window: Rc<FemtoVGWindow>,
    component: Option<ComponentState>,
    scale_factor: f32,
    has_fractional_scale: bool,
}

struct LockConfigureContext {
    scale_factor: f32,
    component_definition: ComponentDefinition,
    compilation_result: Option<Rc<CompilationResult>>,
    platform: Rc<CustomSlintPlatform>,
    callbacks: Vec<LockCallback>,
}

impl ActiveLockSurface {
    fn new(surface: LockSurface, window: Rc<FemtoVGWindow>) -> Self {
        Self {
            has_fractional_scale: surface.fractional_scale().is_some(),
            surface,
            window,
            component: None,
            scale_factor: 1.0,
        }
    }

    fn handle_configure(
        &mut self,
        serial: u32,
        width: u32,
        height: u32,
        context: &LockConfigureContext,
    ) -> Result<()> {
        self.surface.handle_configure(serial, width, height);
        self.scale_factor = context.scale_factor;
        let dimensions = match SurfaceDimensions::calculate(width, height, context.scale_factor) {
            Ok(dimensions) => dimensions,
            Err(err) => {
                info!("Failed to calculate lock surface dimensions: {err}");
                return Ok(());
            }
        };
        let scaling_mode = self.scaling_mode();
        info!(
            "Lock surface dimensions: logical {}x{}, physical {}x{}, scale {}, mode {:?}",
            dimensions.logical_width(),
            dimensions.logical_height(),
            dimensions.physical_width(),
            dimensions.physical_height(),
            context.scale_factor,
            scaling_mode
        );
        self.configure_window(&dimensions, scaling_mode, context.scale_factor);
        self.configure_surface(&dimensions, scaling_mode);

        if self.component.is_none() {
            context.platform.add_window(Rc::clone(&self.window));
            let component = ComponentState::new(
                context.component_definition.clone(),
                context.compilation_result.clone(),
                &self.window,
            )?;
            self.window
                .window()
                .dispatch_event(WindowEvent::WindowActiveChanged(true));
            for callback in &context.callbacks {
                if let Err(err) = callback.apply_to(component.component_instance()) {
                    info!(
                        "Failed to register lock callback '{}': {err}",
                        callback.name
                    );
                } else {
                    info!("Registered lock callback '{}'", callback.name);
                }
            }
            self.component = Some(component);
        }

        RenderableWindow::request_redraw(self.window.as_ref());
        Ok(())
    }

    fn render_frame_if_dirty(&self) -> Result<()> {
        self.window.render_frame_if_dirty()
    }

    fn handle_fractional_scale(&mut self, scale_120ths: u32) {
        let scale_factor = DisplayMetrics::scale_factor_from_120ths(scale_120ths);
        self.scale_factor = scale_factor;
        if self.surface.width() == 0 || self.surface.height() == 0 {
            return;
        }
        let Ok(dimensions) =
            SurfaceDimensions::calculate(self.surface.width(), self.surface.height(), scale_factor)
        else {
            return;
        };
        let scaling_mode = self.scaling_mode();
        self.configure_window(&dimensions, scaling_mode, scale_factor);
        self.configure_surface(&dimensions, scaling_mode);
        RenderableWindow::request_redraw(self.window.as_ref());
    }

    fn apply_callback(&self, callback: &LockCallback) {
        if let Some(component) = self.component.as_ref() {
            if let Err(err) = callback.apply_to(component.component_instance()) {
                info!(
                    "Failed to register lock callback '{}': {err}",
                    callback.name
                );
            }
        }
    }

    fn scaling_mode(&self) -> LockScalingMode {
        if self.surface.has_fractional_scale() && self.surface.has_viewport() {
            LockScalingMode::FractionalWithViewport
        } else if self.surface.has_fractional_scale() {
            LockScalingMode::FractionalOnly
        } else {
            LockScalingMode::Integer
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn configure_window(
        &self,
        dimensions: &SurfaceDimensions,
        mode: LockScalingMode,
        scale_factor: f32,
    ) {
        match mode {
            LockScalingMode::FractionalWithViewport | LockScalingMode::FractionalOnly => {
                RenderableWindow::set_scale_factor(self.window.as_ref(), scale_factor);
                self.window.set_size(WindowSize::Logical(LogicalSize::new(
                    dimensions.logical_width() as f32,
                    dimensions.logical_height() as f32,
                )));
            }
            LockScalingMode::Integer => {
                RenderableWindow::set_scale_factor(self.window.as_ref(), scale_factor);
                self.window
                    .set_size(WindowSize::Physical(slint::PhysicalSize::new(
                        dimensions.physical_width(),
                        dimensions.physical_height(),
                    )));
            }
        }
    }

    fn configure_surface(&self, dimensions: &SurfaceDimensions, mode: LockScalingMode) {
        match mode {
            LockScalingMode::FractionalWithViewport => {
                self.surface.configure_fractional_viewport(
                    dimensions.logical_width(),
                    dimensions.logical_height(),
                );
            }
            LockScalingMode::FractionalOnly | LockScalingMode::Integer => {
                self.surface
                    .configure_buffer_scale(dimensions.buffer_scale());
            }
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn to_logical_position(&self, surface_x: f64, surface_y: f64) -> LogicalPosition {
        if self.has_fractional_scale {
            let x = surface_x as f32;
            let y = surface_y as f32;
            LogicalPosition::new(x, y)
        } else {
            let x = (surface_x / f64::from(self.scale_factor)) as f32;
            let y = (surface_y / f64::from(self.scale_factor)) as f32;
            LogicalPosition::new(x, y)
        }
    }

    fn dispatch_event(&self, event: WindowEvent) {
        self.window.window().dispatch_event(event);
    }

    fn window_rc(&self) -> Rc<FemtoVGWindow> {
        Rc::clone(&self.window)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LockScalingMode {
    FractionalWithViewport,
    FractionalOnly,
    Integer,
}

pub struct SessionLockManager {
    context: Rc<SessionLockContext>,
    session_lock: Option<ExtSessionLockV1>,
    lock_surfaces: HashMap<ObjectId, ActiveLockSurface>,
    state: LockState,
    config: LockConfig,
    component_definition: ComponentDefinition,
    compilation_result: Option<Rc<CompilationResult>>,
    platform: Rc<CustomSlintPlatform>,
    callbacks: Vec<LockCallback>,
    active_pointer_surface_id: Option<ObjectId>,
    keyboard_focus_surface_id: Option<ObjectId>,
    current_pointer_position: LogicalPosition,
    accumulated_axis_x: f32,
    accumulated_axis_y: f32,
}

impl SessionLockManager {
    #[must_use]
    pub fn new(
        context: Rc<SessionLockContext>,
        component_definition: ComponentDefinition,
        compilation_result: Option<Rc<CompilationResult>>,
        platform: Rc<CustomSlintPlatform>,
        config: LockConfig,
    ) -> Self {
        Self {
            context,
            session_lock: None,
            lock_surfaces: HashMap::new(),
            state: LockState::Inactive,
            config,
            component_definition,
            compilation_result,
            platform,
            callbacks: Vec::new(),
            active_pointer_surface_id: None,
            keyboard_focus_surface_id: None,
            current_pointer_position: LogicalPosition::new(0.0, 0.0),
            accumulated_axis_x: 0.0,
            accumulated_axis_y: 0.0,
        }
    }

    #[must_use]
    pub const fn state(&self) -> LockState {
        self.state
    }

    pub fn activate(
        &mut self,
        outputs: impl IntoIterator<Item = WlOutput>,
        queue_handle: &QueueHandle<AppState>,
    ) -> Result<()> {
        if !self.state.can_activate() {
            return Err(LayerShikaError::InvalidInput {
                message: format!("Session lock cannot activate in state {:?}", self.state),
            });
        }

        self.config.validate()?;

        let session_lock = self.context.lock_manager().lock(queue_handle, ());
        self.session_lock = Some(session_lock.clone());
        self.state = LockState::Locking;

        for output in outputs {
            let params = LockSurfaceParams {
                compositor: self.context.compositor(),
                output: &output,
                session_lock: &session_lock,
                fractional_scale_manager: self.context.fractional_scale_manager(),
                viewporter: self.context.viewporter(),
                queue_handle,
            };
            let surface = LockSurface::create(&params);
            let surface_id = surface.surface_id();
            let window = self.create_window(&surface_id)?;
            self.lock_surfaces
                .insert(output.id(), ActiveLockSurface::new(surface, window));
        }

        Ok(())
    }

    pub fn handle_locked(&mut self) {
        if self.state == LockState::Locking {
            info!("Session lock transitioned to Locked");
            self.state = LockState::Locked;
        }
    }

    pub fn deactivate(&mut self) -> Result<()> {
        if !self.state.can_deactivate() {
            return Err(LayerShikaError::InvalidInput {
                message: format!("Session lock cannot deactivate in state {:?}", self.state),
            });
        }

        let Some(session_lock) = self.session_lock.take() else {
            return Err(LayerShikaError::InvalidInput {
                message: "Session lock object missing during deactivate".to_string(),
            });
        };

        for surface in self.lock_surfaces.values() {
            surface.surface.destroy();
        }
        session_lock.unlock_and_destroy();
        self.lock_surfaces.clear();
        self.active_pointer_surface_id = None;
        self.keyboard_focus_surface_id = None;
        self.state = LockState::Unlocking;
        Ok(())
    }

    pub fn handle_finished(&mut self) {
        info!("Session lock finished");
        self.lock_surfaces.clear();
        self.session_lock = None;
        self.state = LockState::Inactive;
        self.active_pointer_surface_id = None;
        self.keyboard_focus_surface_id = None;
    }

    pub fn add_output(
        &mut self,
        output: &WlOutput,
        queue_handle: &QueueHandle<AppState>,
    ) -> Result<()> {
        if self.state != LockState::Locked {
            return Ok(());
        }

        let output_id = output.id();
        if self.lock_surfaces.contains_key(&output_id) {
            return Ok(());
        }

        let Some(session_lock) = self.session_lock.as_ref() else {
            return Err(LayerShikaError::InvalidInput {
                message: "Session lock object missing during output hotplug".to_string(),
            });
        };

        info!("Adding lock surface for output {output_id:?}");
        let params = LockSurfaceParams {
            compositor: self.context.compositor(),
            output,
            session_lock,
            fractional_scale_manager: self.context.fractional_scale_manager(),
            viewporter: self.context.viewporter(),
            queue_handle,
        };
        let surface = LockSurface::create(&params);
        let surface_id = surface.surface_id();
        let window = self.create_window(&surface_id)?;
        self.lock_surfaces
            .insert(output_id, ActiveLockSurface::new(surface, window));
        Ok(())
    }

    pub fn remove_output(&mut self, output_id: &ObjectId) {
        if let Some(surface) = self.lock_surfaces.remove(output_id) {
            let surface_id = surface.surface.surface_id();
            if self.active_pointer_surface_id.as_ref() == Some(&surface_id) {
                self.active_pointer_surface_id = None;
            }
            if self.keyboard_focus_surface_id.as_ref() == Some(&surface_id) {
                self.keyboard_focus_surface_id = None;
            }
            drop(surface);
        }
    }

    fn find_surface_by_lock_surface_id_mut(
        &mut self,
        lock_surface_id: &ObjectId,
    ) -> Option<&mut ActiveLockSurface> {
        self.lock_surfaces
            .values_mut()
            .find(|surface| surface.surface.lock_surface_id() == *lock_surface_id)
    }

    fn find_surface_by_surface_id(&self, surface_id: &ObjectId) -> Option<&ActiveLockSurface> {
        self.lock_surfaces
            .values()
            .find(|surface| surface.surface.surface_id() == *surface_id)
    }

    pub fn handle_configure(
        &mut self,
        lock_surface_id: &ObjectId,
        serial: u32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let context = LockConfigureContext {
            scale_factor: self.config.scale_factor.value(),
            component_definition: self.component_definition.clone(),
            compilation_result: self.compilation_result.clone(),
            platform: Rc::clone(&self.platform),
            callbacks: self.callbacks.clone(),
        };

        let Some(surface) = self.find_surface_by_lock_surface_id_mut(lock_surface_id) else {
            return Ok(());
        };

        surface.handle_configure(serial, width, height, &context)
    }

    pub fn render_frames(&self) -> Result<()> {
        for surface in self.lock_surfaces.values() {
            surface.render_frame_if_dirty()?;
        }
        Ok(())
    }

    pub(crate) fn register_callback(&mut self, callback: LockCallback) {
        for surface in self.lock_surfaces.values() {
            surface.apply_callback(&callback);
        }
        self.callbacks.push(callback);
    }

    pub fn handle_fractional_scale(&mut self, fractional_scale_id: &ObjectId, scale_120ths: u32) {
        for surface in self.lock_surfaces.values_mut() {
            let matches = surface
                .surface
                .fractional_scale()
                .is_some_and(|fs| fs.id() == *fractional_scale_id);
            if matches {
                surface.handle_fractional_scale(scale_120ths);
            }
        }
    }

    pub fn is_lock_surface(&self, surface_id: &ObjectId) -> bool {
        self.find_surface_by_surface_id(surface_id).is_some()
    }

    pub const fn has_active_pointer(&self) -> bool {
        self.active_pointer_surface_id.is_some()
    }

    pub const fn has_keyboard_focus(&self) -> bool {
        self.keyboard_focus_surface_id.is_some()
    }

    pub fn handle_pointer_enter(
        &mut self,
        _serial: u32,
        surface: &WlSurface,
        surface_x: f64,
        surface_y: f64,
    ) -> bool {
        let surface_id = surface.id();
        let (position, window) = {
            let Some(active_surface) = self.find_surface_by_surface_id(&surface_id) else {
                return false;
            };
            (
                active_surface.to_logical_position(surface_x, surface_y),
                active_surface.window_rc(),
            )
        };

        self.active_pointer_surface_id = Some(surface_id.clone());
        self.current_pointer_position = position;
        info!("Lock pointer enter on {:?}", surface_id);
        window
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        true
    }

    pub fn handle_pointer_motion(&mut self, surface_x: f64, surface_y: f64) -> bool {
        let Some(surface_id) = self.active_pointer_surface_id.clone() else {
            return false;
        };
        let (position, window) = {
            let Some(active_surface) = self.find_surface_by_surface_id(&surface_id) else {
                return false;
            };
            (
                active_surface.to_logical_position(surface_x, surface_y),
                active_surface.window_rc(),
            )
        };

        self.current_pointer_position = position;
        window
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        true
    }

    pub fn handle_pointer_leave(&mut self) -> bool {
        let Some(surface_id) = self.active_pointer_surface_id.take() else {
            return false;
        };

        if let Some(active_surface) = self.find_surface_by_surface_id(&surface_id) {
            active_surface.dispatch_event(WindowEvent::PointerExited);
        }
        true
    }

    pub fn handle_pointer_button(
        &mut self,
        _serial: u32,
        button: u32,
        button_state: WEnum<wl_pointer::ButtonState>,
    ) -> bool {
        let Some(surface_id) = self.active_pointer_surface_id.clone() else {
            return false;
        };
        let window = {
            let Some(active_surface) = self.find_surface_by_surface_id(&surface_id) else {
                return false;
            };
            active_surface.window_rc()
        };

        let position = self.current_pointer_position;
        let slint_button = wayland_button_to_slint(button);
        let event = match button_state {
            WEnum::Value(wl_pointer::ButtonState::Pressed) => WindowEvent::PointerPressed {
                button: slint_button,
                position,
            },
            WEnum::Value(wl_pointer::ButtonState::Released) => WindowEvent::PointerReleased {
                button: slint_button,
                position,
            },
            _ => return true,
        };

        info!(
            "Lock pointer button {:?} at {:?} (scale {})",
            button_state,
            position,
            self.config.scale_factor.value()
        );
        window.window().dispatch_event(event);
        true
    }

    pub fn handle_axis_source(&mut self, _axis_source: wl_pointer::AxisSource) -> bool {
        if self.active_pointer_surface_id.is_none() {
            return false;
        }
        true
    }

    pub fn handle_axis(&mut self, axis: wl_pointer::Axis, value: f64) -> bool {
        if self.active_pointer_surface_id.is_none() {
            return false;
        }

        match axis {
            wl_pointer::Axis::HorizontalScroll => {
                #[allow(clippy::cast_possible_truncation)]
                let delta = value as f32;
                self.accumulated_axis_x += delta;
            }
            wl_pointer::Axis::VerticalScroll => {
                #[allow(clippy::cast_possible_truncation)]
                let delta = value as f32;
                self.accumulated_axis_y += delta;
            }
            _ => {}
        }
        true
    }

    pub fn handle_axis_discrete(&mut self, axis: wl_pointer::Axis, discrete: i32) -> bool {
        if self.active_pointer_surface_id.is_none() {
            return false;
        }

        #[allow(clippy::cast_precision_loss)]
        let delta = (discrete as f32) * 60.0;
        match axis {
            wl_pointer::Axis::HorizontalScroll => {
                self.accumulated_axis_x += delta;
            }
            wl_pointer::Axis::VerticalScroll => {
                self.accumulated_axis_y += delta;
            }
            _ => {}
        }
        true
    }

    pub fn handle_axis_stop(&mut self, _axis: wl_pointer::Axis) -> bool {
        self.active_pointer_surface_id.is_some()
    }

    pub fn handle_pointer_frame(&mut self) -> bool {
        let Some(surface_id) = self.active_pointer_surface_id.clone() else {
            return false;
        };
        let delta_x = self.accumulated_axis_x;
        let delta_y = self.accumulated_axis_y;
        self.accumulated_axis_x = 0.0;
        self.accumulated_axis_y = 0.0;

        let window = {
            let Some(active_surface) = self.find_surface_by_surface_id(&surface_id) else {
                return false;
            };
            active_surface.window_rc()
        };

        if delta_x.abs() > f32::EPSILON || delta_y.abs() > f32::EPSILON {
            let position = self.current_pointer_position;
            window
                .window()
                .dispatch_event(WindowEvent::PointerScrolled {
                    position,
                    delta_x,
                    delta_y,
                });
        }

        true
    }

    pub fn handle_keyboard_enter(&mut self, surface: &WlSurface) -> bool {
        let surface_id = surface.id();
        if self.find_surface_by_surface_id(&surface_id).is_some() {
            self.keyboard_focus_surface_id = Some(surface_id);
            return true;
        }
        false
    }

    pub fn handle_keyboard_leave(&mut self, surface: &WlSurface) -> bool {
        let surface_id = surface.id();
        if self.keyboard_focus_surface_id.as_ref() == Some(&surface_id) {
            self.keyboard_focus_surface_id = None;
            return true;
        }
        false
    }

    pub fn handle_keyboard_key(
        &mut self,
        key: u32,
        state: wl_keyboard::KeyState,
        keyboard_state: &mut KeyboardState,
    ) -> bool {
        let Some(surface_id) = self.keyboard_focus_surface_id.clone() else {
            return false;
        };
        let Some(active_surface) = self.find_surface_by_surface_id(&surface_id) else {
            return false;
        };
        let Some(xkb_state) = keyboard_state.xkb_state.as_mut() else {
            return true;
        };

        let keycode = xkb::Keycode::new(key + 8);
        let direction = match state {
            wl_keyboard::KeyState::Pressed => xkb::KeyDirection::Down,
            wl_keyboard::KeyState::Released => xkb::KeyDirection::Up,
            _ => return true,
        };

        xkb_state.update_key(keycode, direction);

        let text = xkb_state.key_get_utf8(keycode);
        let text = if text.is_empty() {
            let keysym = xkb_state.key_get_one_sym(keycode);
            keysym_to_text(keysym)
        } else {
            Some(SharedString::from(text.as_str()))
        };

        let Some(text) = text else {
            return true;
        };

        let event = match state {
            wl_keyboard::KeyState::Pressed => WindowEvent::KeyPressed { text },
            wl_keyboard::KeyState::Released => WindowEvent::KeyReleased { text },
            _ => return true,
        };
        info!("Lock key event {:?}", state);
        active_surface.dispatch_event(event);
        true
    }

    fn create_window(&self, surface_id: &ObjectId) -> Result<Rc<FemtoVGWindow>> {
        let init_size = LogicalSize::new(1.0, 1.0);
        let context = self.context.render_factory().create_context(
            surface_id,
            init_size.to_physical(self.config.scale_factor.value()),
        )?;
        let renderer = FemtoVGRenderer::new(context)
            .map_err(|e| LayerShikaError::FemtoVGRendererCreation { source: e })?;
        let window = FemtoVGWindow::new(renderer);
        RenderableWindow::set_scale_factor(window.as_ref(), self.config.scale_factor.value());
        window.set_size(WindowSize::Logical(init_size));
        window.set_position(WindowPosition::Logical(LogicalPosition::new(0., 0.)));
        Ok(window)
    }
}
