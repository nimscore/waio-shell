use std::rc::Rc;
use super::surface_builder::WindowStateBuilder;
use super::event_router::EventRouter;
use super::popup_coordinator::PopupCoordinator;
use super::popup_manager::PopupManager;
use super::scale_coordinator::{ScaleCoordinator, SharedPointerSerial};
use super::window_renderer::{WindowRenderer, WindowRendererParams};
use crate::wayland::managed_proxies::{
    ManagedWlPointer, ManagedWlSurface, ManagedZwlrLayerSurfaceV1,
    ManagedWpFractionalScaleV1, ManagedWpViewport,
};
use crate::wayland::services::popup_service::PopupService;
use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::errors::{LayerShikaError, Result};
use core::result::Result as CoreResult;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::ports::windowing::RuntimeStatePort;
use slint::{LogicalPosition, PhysicalSize, ComponentHandle};
use slint::platform::WindowEvent;
use slint_interpreter::{ComponentInstance, CompilationResult};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use wayland_client::{protocol::wl_surface::WlSurface, Proxy};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

pub struct WindowState {
    component_instance: ComponentInstance,
    compilation_result: Option<Rc<CompilationResult>>,
    #[allow(dead_code)]
    pointer: ManagedWlPointer,
    renderer: WindowRenderer,
    event_router: EventRouter,
    popup_coordinator: PopupCoordinator,
    scale_coordinator: ScaleCoordinator,
    output_size: PhysicalSize,
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
        let surface = ManagedWlSurface::new(Rc::clone(&surface_rc), Rc::clone(&connection));
        let pointer = ManagedWlPointer::new(pointer_rc, connection);

        let has_fractional_scale = fractional_scale.is_some();
        let size = builder.size.unwrap_or_default();

        let renderer = WindowRenderer::new(WindowRendererParams {
            window: Rc::clone(&window),
            surface,
            layer_surface,
            viewport,
            fractional_scale,
            height: builder.height,
            exclusive_zone: builder.exclusive_zone,
            size,
        });

        let main_surface_id = (*surface_rc).id();
        let event_router = EventRouter::new(Rc::clone(&window), main_surface_id);
        let popup_coordinator = PopupCoordinator::new();
        let scale_coordinator = ScaleCoordinator::new(builder.scale_factor, has_fractional_scale);

        Ok(Self {
            component_instance,
            compilation_result: builder.compilation_result,
            pointer,
            renderer,
            event_router,
            popup_coordinator,
            scale_coordinator,
            output_size: builder.output_size.unwrap_or_default(),
        })
    }

    pub fn update_size(&mut self, width: u32, height: u32) {
        let scale_factor = self.scale_coordinator.scale_factor();
        self.renderer.update_size(width, height, scale_factor);
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        self.scale_coordinator
            .set_current_pointer_position(physical_x, physical_y);
    }

    pub fn size(&self) -> PhysicalSize {
        self.renderer.size()
    }

    pub fn current_pointer_position(&self) -> LogicalPosition {
        self.scale_coordinator.current_pointer_position()
    }

    pub(crate) fn window(&self) -> Rc<FemtoVGWindow> {
        Rc::clone(self.renderer.window())
    }

    pub(crate) fn layer_surface(&self) -> Rc<ZwlrLayerSurfaceV1> {
        self.renderer.layer_surface()
    }

    pub fn height(&self) -> u32 {
        self.renderer.height()
    }

    pub fn set_output_size(&mut self, output_size: PhysicalSize) {
        self.output_size = output_size;
        self.popup_coordinator.update_output_size(output_size);
    }

    pub const fn output_size(&self) -> PhysicalSize {
        self.output_size
    }

    pub const fn component_instance(&self) -> &ComponentInstance {
        &self.component_instance
    }

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.compilation_result.as_ref().map(Rc::clone)
    }

    pub fn render_frame_if_dirty(&self) -> Result<()> {
        self.renderer.render_frame_if_dirty()
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_factor(&mut self, scale_120ths: u32) {
        let new_scale_factor = self.scale_coordinator.update_scale_factor(scale_120ths);

        self.popup_coordinator.update_scale_factor(new_scale_factor);

        let current_logical_size = self.renderer.logical_size();
        if current_logical_size.width > 0 && current_logical_size.height > 0 {
            self.update_size(current_logical_size.width, current_logical_size.height);
        }
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_coordinator.scale_factor()
    }

    pub fn last_pointer_serial(&self) -> u32 {
        self.scale_coordinator.last_pointer_serial()
    }

    pub fn set_last_pointer_serial(&mut self, serial: u32) {
        self.scale_coordinator.set_last_pointer_serial(serial);
    }

    pub fn set_shared_pointer_serial(&mut self, shared_serial: Rc<SharedPointerSerial>) {
        self.scale_coordinator
            .set_shared_pointer_serial(shared_serial);
    }

    pub fn set_popup_service(&mut self, popup_service: Rc<PopupService>) {
        self.event_router
            .set_popup_service(Rc::clone(&popup_service));
        self.popup_coordinator.set_popup_service(popup_service);
    }

    pub fn set_popup_manager(&mut self, popup_manager: Rc<PopupManager>) {
        self.popup_coordinator.set_popup_manager(popup_manager);
    }

    pub fn find_window_for_surface(&mut self, surface: &WlSurface) {
        self.event_router.find_window_for_surface(surface);
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent) {
        self.event_router.dispatch_to_active_window(event);
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_for_fractional_scale_object(
        &mut self,
        fractional_scale_proxy: &WpFractionalScaleV1,
        scale_120ths: u32,
    ) {
        let fractional_scale_id = fractional_scale_proxy.id();

        if let Some(main_fractional_scale) = self.renderer.fractional_scale() {
            if (**main_fractional_scale.inner()).id() == fractional_scale_id {
                self.update_scale_factor(scale_120ths);
                return;
            }
        }

        self.popup_coordinator
            .update_scale_for_fractional_scale_object(fractional_scale_proxy, scale_120ths);
    }

    pub fn clear_active_window(&mut self) {
        self.popup_coordinator.clear_active_window();
    }

    pub fn clear_active_window_if_popup(&mut self, popup_key: usize) {
        self.popup_coordinator
            .clear_active_window_if_popup(popup_key);
    }

    pub fn popup_service(&self) -> &Option<Rc<PopupService>> {
        self.popup_coordinator.popup_service()
    }

    pub fn popup_manager(&self) -> Option<Rc<PopupManager>> {
        self.popup_coordinator.popup_manager()
    }
}

impl RuntimeStatePort for WindowState {
    fn render_frame_if_dirty(&self) -> CoreResult<(), DomainError> {
        WindowState::render_frame_if_dirty(self).map_err(|e| DomainError::Adapter {
            source: Box::new(e),
        })
    }
}
