use std::rc::Rc;
use std::cell::RefCell;
use super::surface_builder::WindowStateBuilder;
use super::dimensions::SurfaceDimensionsExt;
use super::popup_manager::PopupManager;
use crate::wayland::managed_proxies::{
    ManagedWlPointer, ManagedWlSurface, ManagedZwlrLayerSurfaceV1,
    ManagedWpFractionalScaleV1, ManagedWpViewport,
};
use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::errors::{LayerShikaError, Result};
use core::result::Result as CoreResult;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::ports::windowing::RuntimeStatePort;
use layer_shika_domain::surface_dimensions::SurfaceDimensions;
use log::info;
use slint::{LogicalPosition, PhysicalSize, ComponentHandle};
use slint::platform::{WindowAdapter, WindowEvent};
use slint_interpreter::{ComponentInstance, CompilationResult};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use wayland_client::{protocol::wl_surface::WlSurface, Proxy};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

pub struct SharedPointerSerial {
    serial: RefCell<u32>,
}

impl Default for SharedPointerSerial {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedPointerSerial {
    pub const fn new() -> Self {
        Self {
            serial: RefCell::new(0),
        }
    }

    pub fn update(&self, serial: u32) {
        *self.serial.borrow_mut() = serial;
    }

    pub fn get(&self) -> u32 {
        *self.serial.borrow()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScalingMode {
    FractionalWithViewport,
    FractionalOnly,
    Integer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveWindow {
    Main,
    Popup(usize),
}

struct MutableWindowState {
    size: PhysicalSize,
    logical_size: PhysicalSize,
    output_size: PhysicalSize,
    current_pointer_position: LogicalPosition,
    last_pointer_serial: u32,
    shared_pointer_serial: Option<Rc<SharedPointerSerial>>,
    scale_factor: f32,
    active_window: Option<ActiveWindow>,
}

impl MutableWindowState {
    #[allow(clippy::cast_possible_truncation)]
    fn set_pointer_position(&mut self, physical_x: f64, physical_y: f64, fractional_scale: bool) {
        let logical_position = if fractional_scale {
            LogicalPosition::new(physical_x as f32, physical_y as f32)
        } else {
            let scale_factor = self.scale_factor;
            LogicalPosition::new(
                (physical_x / f64::from(scale_factor)) as f32,
                (physical_y / f64::from(scale_factor)) as f32,
            )
        };
        self.current_pointer_position = logical_position;
    }
}

pub struct WindowState {
    component_instance: ComponentInstance,
    compilation_result: Option<Rc<CompilationResult>>,
    viewport: Option<ManagedWpViewport>,
    fractional_scale: Option<ManagedWpFractionalScaleV1>,
    layer_surface: ManagedZwlrLayerSurfaceV1,
    surface: ManagedWlSurface,
    #[allow(dead_code)]
    pointer: ManagedWlPointer,
    window: Rc<FemtoVGWindow>,
    height: u32,
    exclusive_zone: i32,
    popup_manager: Option<Rc<PopupManager>>,
    mutable_state: RefCell<MutableWindowState>,
}

impl WindowState {
    pub fn new(builder: WindowStateBuilder) -> Result<Self> {
        let component_definition =
            builder
                .component_definition
                .ok_or_else(|| LayerShikaError::InvalidInput {
                    message: "Component definition is required".into(),
                })?;
        let window = builder
            .window
            .ok_or_else(|| LayerShikaError::InvalidInput {
                message: "Window is required".into(),
            })?;
        let component_instance = component_definition
            .create()
            .map_err(|e| LayerShikaError::SlintComponentCreation { source: e })?;
        component_instance
            .show()
            .map_err(|e| LayerShikaError::SlintComponentCreation { source: e })?;

        window.request_redraw();

        let connection = builder
            .connection
            .ok_or_else(|| LayerShikaError::InvalidInput {
                message: "Connection is required".into(),
            })?;

        let surface_rc = builder
            .surface
            .ok_or_else(|| LayerShikaError::InvalidInput {
                message: "Surface is required".into(),
            })?;
        let layer_surface_rc =
            builder
                .layer_surface
                .ok_or_else(|| LayerShikaError::InvalidInput {
                    message: "Layer surface is required".into(),
                })?;
        let pointer_rc = builder
            .pointer
            .ok_or_else(|| LayerShikaError::InvalidInput {
                message: "Pointer is required".into(),
            })?;

        let viewport = builder
            .viewport
            .map(|vp| ManagedWpViewport::new(vp, Rc::clone(&connection)));
        let fractional_scale = builder
            .fractional_scale
            .map(|fs| ManagedWpFractionalScaleV1::new(fs, Rc::clone(&connection)));
        let layer_surface =
            ManagedZwlrLayerSurfaceV1::new(layer_surface_rc, Rc::clone(&connection));
        let surface = ManagedWlSurface::new(surface_rc, Rc::clone(&connection));
        let pointer = ManagedWlPointer::new(pointer_rc, connection);

        Ok(Self {
            component_instance,
            compilation_result: builder.compilation_result,
            viewport,
            fractional_scale,
            layer_surface,
            surface,
            pointer,
            window,
            height: builder.height,
            exclusive_zone: builder.exclusive_zone,
            popup_manager: None,
            mutable_state: RefCell::new(MutableWindowState {
                size: builder.size.unwrap_or_default(),
                logical_size: PhysicalSize::default(),
                output_size: builder.output_size.unwrap_or_default(),
                current_pointer_position: LogicalPosition::default(),
                last_pointer_serial: 0,
                shared_pointer_serial: None,
                scale_factor: builder.scale_factor,
                active_window: None,
            }),
        })
    }

    const fn determine_scaling_mode(&self) -> ScalingMode {
        if self.fractional_scale.is_some() && self.viewport.is_some() {
            ScalingMode::FractionalWithViewport
        } else if self.fractional_scale.is_some() {
            ScalingMode::FractionalOnly
        } else {
            ScalingMode::Integer
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn configure_slint_window(&self, dimensions: &SurfaceDimensions, mode: ScalingMode) {
        let state = self.mutable_state.borrow();
        match mode {
            ScalingMode::FractionalWithViewport => {
                self.window.set_scale_factor(state.scale_factor);
                self.window
                    .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
                        dimensions.logical_width as f32,
                        dimensions.logical_height as f32,
                    )));
            }
            ScalingMode::FractionalOnly => {
                self.window.set_scale_factor(dimensions.buffer_scale as f32);
                self.window
                    .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
                        dimensions.logical_width as f32,
                        dimensions.logical_height as f32,
                    )));
            }
            ScalingMode::Integer => {
                self.window.set_scale_factor(state.scale_factor);
                self.window
                    .set_size(slint::WindowSize::Physical(dimensions.physical_size()));
            }
        }
    }

    #[allow(clippy::cast_possible_wrap)]
    fn configure_wayland_surface(&self, dimensions: &SurfaceDimensions, mode: ScalingMode) {
        match mode {
            ScalingMode::FractionalWithViewport => {
                self.surface.set_buffer_scale(1);
                if let Some(viewport) = &self.viewport {
                    viewport.set_destination(
                        dimensions.logical_width as i32,
                        dimensions.logical_height as i32,
                    );
                }
            }
            ScalingMode::FractionalOnly | ScalingMode::Integer => {
                self.surface.set_buffer_scale(dimensions.buffer_scale);
            }
        }

        self.layer_surface
            .set_size(dimensions.logical_width, dimensions.logical_height);
        self.layer_surface.set_exclusive_zone(self.exclusive_zone);
        self.surface.commit();
    }

    pub fn update_size(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            info!("Skipping update_size with zero dimension: {width}x{height}");
            return;
        }

        let scale_factor = self.scale_factor();
        let dimensions = SurfaceDimensions::calculate(width, height, scale_factor);
        let scaling_mode = self.determine_scaling_mode();

        info!(
            "Updating window size: logical {}x{}, physical {}x{}, scale {}, buffer_scale {}, mode {:?}",
            dimensions.logical_width,
            dimensions.logical_height,
            dimensions.physical_width,
            dimensions.physical_height,
            scale_factor,
            dimensions.buffer_scale,
            scaling_mode
        );

        self.configure_slint_window(&dimensions, scaling_mode);
        self.configure_wayland_surface(&dimensions, scaling_mode);

        info!("Window physical size: {:?}", self.window.size());

        let mut state = self.mutable_state.borrow_mut();
        state.size = dimensions.physical_size();
        state.logical_size = dimensions.logical_size();
        self.window.request_redraw();
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        let has_fractional_scale = self.fractional_scale.is_some();
        let mut state = self.mutable_state.borrow_mut();
        state.set_pointer_position(physical_x, physical_y, has_fractional_scale);
    }

    pub fn size(&self) -> PhysicalSize {
        self.mutable_state.borrow().size
    }

    pub fn current_pointer_position(&self) -> LogicalPosition {
        self.mutable_state.borrow().current_pointer_position
    }

    pub(crate) fn window(&self) -> Rc<FemtoVGWindow> {
        Rc::clone(&self.window)
    }

    pub(crate) fn layer_surface(&self) -> Rc<ZwlrLayerSurfaceV1> {
        Rc::clone(self.layer_surface.inner())
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn set_output_size(&mut self, output_size: PhysicalSize) {
        self.mutable_state.borrow_mut().output_size = output_size;
        if let Some(popup_manager) = &self.popup_manager {
            popup_manager.update_output_size(output_size);
        }
    }

    pub fn output_size(&self) -> PhysicalSize {
        self.mutable_state.borrow().output_size
    }

    pub const fn component_instance(&self) -> &ComponentInstance {
        &self.component_instance
    }

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.compilation_result.as_ref().map(Rc::clone)
    }

    pub fn render_frame_if_dirty(&self) -> Result<()> {
        self.window.render_frame_if_dirty()
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_factor(&mut self, scale_120ths: u32) {
        let new_scale_factor = scale_120ths as f32 / 120.0;
        let old_scale_factor = self.scale_factor();
        info!(
            "Updating scale factor from {} to {} ({}x)",
            old_scale_factor, new_scale_factor, scale_120ths
        );
        self.mutable_state.borrow_mut().scale_factor = new_scale_factor;

        if let Some(popup_manager) = &self.popup_manager {
            popup_manager.update_scale_factor(new_scale_factor);
        }

        let current_logical_size = self.mutable_state.borrow().logical_size;
        if current_logical_size.width > 0 && current_logical_size.height > 0 {
            self.update_size(current_logical_size.width, current_logical_size.height);
        }
    }

    pub fn scale_factor(&self) -> f32 {
        self.mutable_state.borrow().scale_factor
    }

    pub fn last_pointer_serial(&self) -> u32 {
        self.mutable_state.borrow().last_pointer_serial
    }

    pub fn set_last_pointer_serial(&mut self, serial: u32) {
        let mut state = self.mutable_state.borrow_mut();
        state.last_pointer_serial = serial;
        if let Some(ref shared_serial) = state.shared_pointer_serial {
            shared_serial.update(serial);
        }
    }

    pub fn set_shared_pointer_serial(&mut self, shared_serial: Rc<SharedPointerSerial>) {
        self.mutable_state.borrow_mut().shared_pointer_serial = Some(shared_serial);
    }

    pub fn set_popup_manager(&mut self, popup_manager: Rc<PopupManager>) {
        self.popup_manager = Some(popup_manager);
    }

    pub fn find_window_for_surface(&mut self, surface: &WlSurface) {
        let surface_id = surface.id();

        if (**self.surface.inner()).id() == surface_id {
            self.mutable_state.borrow_mut().active_window = Some(ActiveWindow::Main);
            return;
        }

        if let Some(popup_manager) = &self.popup_manager {
            if let Some(popup_key) = popup_manager.find_popup_key_by_surface_id(&surface_id) {
                self.mutable_state.borrow_mut().active_window =
                    Some(ActiveWindow::Popup(popup_key));
                return;
            }
        }

        self.mutable_state.borrow_mut().active_window = None;
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent) {
        let active_window = self.mutable_state.borrow().active_window;
        match active_window {
            Some(ActiveWindow::Main) => {
                self.window.window().dispatch_event(event);
            }
            Some(ActiveWindow::Popup(index)) => {
                if let Some(popup_manager) = &self.popup_manager {
                    if let Some(popup_window) = popup_manager.get_popup_window(index) {
                        popup_window.dispatch_event(event);
                    }
                }
            }
            None => {}
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_for_fractional_scale_object(
        &mut self,
        fractional_scale_proxy: &WpFractionalScaleV1,
        scale_120ths: u32,
    ) {
        let fractional_scale_id = fractional_scale_proxy.id();

        if let Some(ref main_fractional_scale) = self.fractional_scale {
            if (**main_fractional_scale.inner()).id() == fractional_scale_id {
                self.update_scale_factor(scale_120ths);
                return;
            }
        }

        if let Some(popup_manager) = &self.popup_manager {
            if let Some(popup_key) =
                popup_manager.find_popup_key_by_fractional_scale_id(&fractional_scale_id)
            {
                if let Some(popup_window) = popup_manager.get_popup_window(popup_key) {
                    let new_scale_factor = scale_120ths as f32 / 120.0;
                    info!("Updating popup scale factor to {new_scale_factor} ({scale_120ths}x)");
                    popup_window.set_scale_factor(new_scale_factor);
                    popup_window.request_redraw();
                }
            }
        }
    }

    pub fn clear_active_window(&mut self) {
        self.mutable_state.borrow_mut().active_window = None;
    }

    pub fn clear_active_window_if_popup(&mut self, popup_key: usize) {
        let mut state = self.mutable_state.borrow_mut();
        if state.active_window == Some(ActiveWindow::Popup(popup_key)) {
            state.active_window = None;
        }
    }

    pub const fn popup_manager(&self) -> &Option<Rc<PopupManager>> {
        &self.popup_manager
    }
}

impl RuntimeStatePort for WindowState {
    fn render_frame_if_dirty(&self) -> CoreResult<(), DomainError> {
        WindowState::render_frame_if_dirty(self).map_err(|e| DomainError::Adapter {
            source: Box::new(e),
        })
    }
}
