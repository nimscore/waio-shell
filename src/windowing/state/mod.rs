use std::rc::Rc;
use builder::WindowStateBuilder;
use log::info;
use slint::{LogicalPosition, PhysicalSize, ComponentHandle};
use slint_interpreter::ComponentInstance;
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use crate::rendering::femtovg_window::FemtoVGWindow;
use crate::errors::{LayerShikaError, Result};

pub mod builder;
pub mod dispatches;

pub struct WindowState {
    component_instance: ComponentInstance,
    surface: Rc<WlSurface>,
    layer_surface: Rc<ZwlrLayerSurfaceV1>,
    fractional_scale: Option<Rc<WpFractionalScaleV1>>,
    viewport: Option<Rc<WpViewport>>,
    size: PhysicalSize,
    logical_size: PhysicalSize,
    output_size: PhysicalSize,
    window: Rc<FemtoVGWindow>,
    current_pointer_position: LogicalPosition,
    scale_factor: f32,
    height: u32,
    exclusive_zone: i32,
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

        Ok(Self {
            component_instance,
            surface: builder
                .surface
                .ok_or_else(|| LayerShikaError::InvalidInput("Surface is required".into()))?,
            layer_surface: builder
                .layer_surface
                .ok_or_else(|| LayerShikaError::InvalidInput("Layer surface is required".into()))?,
            fractional_scale: builder.fractional_scale,
            viewport: builder.viewport,
            size: builder.size.unwrap_or_default(),
            logical_size: PhysicalSize::default(),
            output_size: builder.output_size.unwrap_or_default(),
            window,
            current_pointer_position: LogicalPosition::default(),
            scale_factor: builder.scale_factor,
            height: builder.height,
            exclusive_zone: builder.exclusive_zone,
        })
    }

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_wrap)]
    pub fn update_size(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            info!("Skipping update_size with zero dimension: {width}x{height}");
            return;
        }

        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_precision_loss)]
        let physical_width = (width as f32 * self.scale_factor).round() as u32;
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_precision_loss)]
        let physical_height = (height as f32 * self.scale_factor).round() as u32;

        let new_physical_size = PhysicalSize::new(physical_width, physical_height);
        let new_logical_size = PhysicalSize::new(width, height);

        info!(
            "Updating window size: buffer {}x{}, physical {}x{}, scale {}, has_viewport: {}",
            width,
            height,
            physical_width,
            physical_height,
            self.scale_factor,
            self.viewport.is_some()
        );

        if self.fractional_scale.is_some() && self.viewport.is_some() {
            self.surface.set_buffer_scale(1);
            self.window
                .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
                    width as f32,
                    height as f32,
                )));
            self.window.set_scale_factor(self.scale_factor);

            if let Some(viewport) = &self.viewport {
                viewport.set_destination(width as i32, height as i32);
            }
        } else if self.fractional_scale.is_some() {
            #[allow(clippy::cast_possible_truncation)]
            let buffer_scale = self.scale_factor.round() as i32;
            self.surface.set_buffer_scale(buffer_scale);

            self.window
                .set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
                    width as f32,
                    height as f32,
                )));
            #[allow(clippy::cast_precision_loss)]
            self.window.set_scale_factor(buffer_scale as f32);
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let buffer_scale = self.scale_factor.round() as i32;
            self.surface.set_buffer_scale(buffer_scale);

            self.window
                .set_size(slint::WindowSize::Physical(new_physical_size));
            self.window.set_scale_factor(self.scale_factor);
        }

        info!("Window physical size: {:?}", self.window.size());

        self.layer_surface.set_size(width, height);
        self.layer_surface.set_exclusive_zone(self.exclusive_zone);
        self.surface.commit();

        self.size = new_physical_size;
        self.logical_size = new_logical_size;
        self.window.request_redraw();
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        let logical_position = if self.fractional_scale.is_some() {
            LogicalPosition::new(physical_x as f32, physical_y as f32)
        } else {
            let scale_factor = self.scale_factor;
            LogicalPosition::new(
                physical_x as f32 / scale_factor,
                physical_y as f32 / scale_factor,
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
        Rc::clone(&self.layer_surface)
    }

    pub fn surface(&self) -> Rc<WlSurface> {
        Rc::clone(&self.surface)
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

    pub fn update_scale_factor(&mut self, scale_120ths: u32) {
        #[allow(clippy::cast_precision_loss)]
        let new_scale_factor = scale_120ths as f32 / 120.0;
        info!(
            "Updating scale factor from {} to {} ({}x)",
            self.scale_factor, new_scale_factor, scale_120ths
        );
        self.scale_factor = new_scale_factor;

        let current_logical_size = self.logical_size;
        if current_logical_size.width > 0 && current_logical_size.height > 0 {
            self.update_size(current_logical_size.width, current_logical_size.height);
        }
    }

    pub const fn scale_factor(&self) -> f32 {
        self.scale_factor
    }
}
