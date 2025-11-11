use std::rc::Rc;
use crate::errors::Result;
use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::wayland::surfaces::window_renderer::{WindowRenderer, WindowRendererParams};
use crate::wayland::surfaces::event_bus::EventBus;
use crate::wayland::surfaces::window_events::WindowStateEvent;
use slint::PhysicalSize;
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use crate::wayland::managed_proxies::ManagedWpFractionalScaleV1;

pub struct RenderingState {
    renderer: WindowRenderer,
    event_bus: EventBus,
}

impl RenderingState {
    #[must_use]
    pub fn new(params: WindowRendererParams) -> Self {
        Self {
            renderer: WindowRenderer::new(params),
            event_bus: EventBus::new(),
        }
    }

    pub fn set_event_bus(&mut self, event_bus: EventBus) {
        self.event_bus = event_bus;
    }

    pub fn render_frame_if_dirty(&self) -> Result<()> {
        self.renderer.render_frame_if_dirty()
    }

    pub fn update_size(&mut self, width: u32, height: u32, scale_factor: f32) {
        self.renderer.update_size(width, height, scale_factor);

        self.event_bus.publish(&WindowStateEvent::SizeChanged {
            logical_width: width,
            logical_height: height,
        });
    }

    pub const fn size(&self) -> PhysicalSize {
        self.renderer.size()
    }

    pub const fn logical_size(&self) -> PhysicalSize {
        self.renderer.logical_size()
    }

    pub const fn height(&self) -> u32 {
        self.renderer.height()
    }

    pub const fn window(&self) -> &Rc<FemtoVGWindow> {
        self.renderer.window()
    }

    pub fn layer_surface(&self) -> Rc<ZwlrLayerSurfaceV1> {
        self.renderer.layer_surface()
    }

    pub const fn fractional_scale(&self) -> &Option<ManagedWpFractionalScaleV1> {
        self.renderer.fractional_scale()
    }
}
