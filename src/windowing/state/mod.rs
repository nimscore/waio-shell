use std::rc::Rc;
use builder::WindowStateBuilder;
use log::info;
use slint::{LogicalPosition, PhysicalSize, ComponentHandle};
use slint::platform::{WindowAdapter, WindowEvent};
use slint_interpreter::ComponentInstance;
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use wayland_client::{protocol::{wl_output::WlOutput, wl_surface::WlSurface}, Proxy};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use crate::rendering::femtovg_window::FemtoVGWindow;
use crate::errors::{LayerShikaError, Result};
use crate::windowing::surface_dimensions::SurfaceDimensions;
use crate::windowing::popup_manager::PopupManager;
use crate::windowing::proxies::{
    ManagedWlPointer, ManagedWlSurface, ManagedZwlrLayerSurfaceV1,
    ManagedWpFractionalScaleV1, ManagedWpViewport,
};

pub mod builder;
pub mod dispatches;

#[derive(Debug)]
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

pub struct WindowState {
    component_instance: ComponentInstance,
    viewport: Option<ManagedWpViewport>,
    fractional_scale: Option<ManagedWpFractionalScaleV1>,
    layer_surface: ManagedZwlrLayerSurfaceV1,
    surface: ManagedWlSurface,
    #[allow(dead_code)]
    pointer: ManagedWlPointer,
    #[allow(dead_code)]
    output: Rc<WlOutput>,
    size: PhysicalSize,
    logical_size: PhysicalSize,
    output_size: PhysicalSize,
    window: Rc<FemtoVGWindow>,
    current_pointer_position: LogicalPosition,
    last_pointer_serial: u32,
    scale_factor: f32,
    height: u32,
    exclusive_zone: i32,
    popup_manager: Option<Rc<PopupManager>>,
    active_window: Option<ActiveWindow>,
}

impl WindowState {
    pub fn new(builder: WindowStateBuilder) -> Result<Self> {
        let component_definition = builder.component_definition.ok_or_else(|| {
            LayerShikaError::InvalidInput("Component definition is required".into())
        })?;
        let window = builder
            .window
            .ok_or_else(|| LayerShikaError::InvalidInput("Window is required".into()))?;
        let component_instance = component_definition
            .create()
            .map_err(|e| LayerShikaError::SlintComponentCreation(e.to_string()))?;
        component_instance
            .show()
            .map_err(|e| LayerShikaError::SlintComponentCreation(e.to_string()))?;

        window.request_redraw();

        let connection = builder
            .connection
            .ok_or_else(|| LayerShikaError::InvalidInput("Connection is required".into()))?;

        let surface_rc = builder
            .surface
            .ok_or_else(|| LayerShikaError::InvalidInput("Surface is required".into()))?;
        let layer_surface_rc = builder
            .layer_surface
            .ok_or_else(|| LayerShikaError::InvalidInput("Layer surface is required".into()))?;
        let pointer_rc = builder
            .pointer
            .ok_or_else(|| LayerShikaError::InvalidInput("Pointer is required".into()))?;

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
            viewport,
            fractional_scale,
            layer_surface,
            surface,
            pointer,
            output: builder
                .output
                .ok_or_else(|| LayerShikaError::InvalidInput("Output is required".into()))?,
            size: builder.size.unwrap_or_default(),
            logical_size: PhysicalSize::default(),
            output_size: builder.output_size.unwrap_or_default(),
            window,
            current_pointer_position: LogicalPosition::default(),
            last_pointer_serial: 0,
            scale_factor: builder.scale_factor,
            height: builder.height,
            exclusive_zone: builder.exclusive_zone,
            popup_manager: None,
            active_window: None,
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
    fn configure_slint_window(&self, dimensions: &SurfaceDimensions, mode: &ScalingMode) {
        match mode {
            ScalingMode::FractionalWithViewport => {
                self.window
                    .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
                        dimensions.logical_width as f32,
                        dimensions.logical_height as f32,
                    )));
                self.window.set_scale_factor(self.scale_factor);
            }
            ScalingMode::FractionalOnly => {
                self.window
                    .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
                        dimensions.logical_width as f32,
                        dimensions.logical_height as f32,
                    )));
                self.window.set_scale_factor(dimensions.buffer_scale as f32);
            }
            ScalingMode::Integer => {
                self.window
                    .set_size(slint::WindowSize::Physical(dimensions.physical_size()));
                self.window.set_scale_factor(self.scale_factor);
            }
        }
    }

    #[allow(clippy::cast_possible_wrap)]
    fn configure_wayland_surface(&self, dimensions: &SurfaceDimensions, mode: &ScalingMode) {
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

        let dimensions = SurfaceDimensions::calculate(width, height, self.scale_factor);
        let scaling_mode = self.determine_scaling_mode();

        info!(
            "Updating window size: logical {}x{}, physical {}x{}, scale {}, buffer_scale {}, mode {:?}",
            dimensions.logical_width,
            dimensions.logical_height,
            dimensions.physical_width,
            dimensions.physical_height,
            self.scale_factor,
            dimensions.buffer_scale,
            scaling_mode
        );

        self.configure_slint_window(&dimensions, &scaling_mode);
        self.configure_wayland_surface(&dimensions, &scaling_mode);

        info!("Window physical size: {:?}", self.window.size());

        self.size = dimensions.physical_size();
        self.logical_size = dimensions.logical_size();
        self.window.request_redraw();
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        let logical_position = if self.fractional_scale.is_some() {
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

    pub const fn size(&self) -> &PhysicalSize {
        &self.size
    }

    pub const fn current_pointer_position(&self) -> &LogicalPosition {
        &self.current_pointer_position
    }

    pub fn window(&self) -> Rc<FemtoVGWindow> {
        Rc::clone(&self.window)
    }

    pub fn layer_surface(&self) -> Rc<ZwlrLayerSurfaceV1> {
        Rc::clone(self.layer_surface.inner())
    }

    pub fn surface(&self) -> Rc<WlSurface> {
        Rc::clone(self.surface.inner())
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn set_output_size(&mut self, output_size: PhysicalSize) {
        self.output_size = output_size;
    }

    pub const fn output_size(&self) -> &PhysicalSize {
        &self.output_size
    }

    pub const fn component_instance(&self) -> &ComponentInstance {
        &self.component_instance
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_factor(&mut self, scale_120ths: u32) {
        let new_scale_factor = scale_120ths as f32 / 120.0;
        info!(
            "Updating scale factor from {} to {} ({}x)",
            self.scale_factor, new_scale_factor, scale_120ths
        );
        self.scale_factor = new_scale_factor;

        if let Some(popup_manager) = &self.popup_manager {
            popup_manager.update_scale_factor(new_scale_factor);
        }

        let current_logical_size = self.logical_size;
        if current_logical_size.width > 0 && current_logical_size.height > 0 {
            self.update_size(current_logical_size.width, current_logical_size.height);
        }
    }

    pub const fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub const fn last_pointer_serial(&self) -> u32 {
        self.last_pointer_serial
    }

    pub const fn set_last_pointer_serial(&mut self, serial: u32) {
        self.last_pointer_serial = serial;
    }

    pub fn set_popup_manager(&mut self, popup_manager: Rc<PopupManager>) {
        self.popup_manager = Some(popup_manager);
    }

    pub fn find_window_for_surface(&mut self, surface: &WlSurface) {
        let surface_id = surface.id();

        if (**self.surface.inner()).id() == surface_id {
            self.active_window = Some(ActiveWindow::Main);
            return;
        }

        if let Some(popup_manager) = &self.popup_manager {
            if let Some(popup_index) = popup_manager.find_popup_index_by_surface_id(&surface_id) {
                self.active_window = Some(ActiveWindow::Popup(popup_index));
                return;
            }
        }

        self.active_window = None;
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent) {
        match self.active_window {
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
            if let Some(popup_index) =
                popup_manager.find_popup_index_by_fractional_scale_id(&fractional_scale_id)
            {
                if let Some(popup_window) = popup_manager.get_popup_window(popup_index) {
                    let new_scale_factor = scale_120ths as f32 / 120.0;
                    info!("Updating popup scale factor to {new_scale_factor} ({scale_120ths}x)");
                    popup_window.set_scale_factor(new_scale_factor);
                    popup_window.request_redraw();
                }
            }
        }
    }
}
