use std::rc::Rc;
use std::cell::RefCell;
use super::surface_builder::WindowStateBuilder;
use super::component_state::ComponentState;
use super::rendering_state::RenderingState;
use super::event_context::{EventContext, SharedPointerSerial};
use super::popup_manager::PopupManager;
use super::window_renderer::WindowRendererParams;
use super::display_metrics::{DisplayMetrics, SharedDisplayMetrics};
use crate::wayland::managed_proxies::{
    ManagedWlPointer, ManagedWlSurface, ManagedZwlrLayerSurfaceV1,
    ManagedWpFractionalScaleV1, ManagedWpViewport,
};
use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::errors::{LayerShikaError, Result};
use core::result::Result as CoreResult;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::ports::windowing::RuntimeStatePort;
use slint::{LogicalPosition, PhysicalSize};
use slint::platform::WindowEvent;
use slint_interpreter::{ComponentInstance, CompilationResult};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use wayland_client::{protocol::wl_surface::WlSurface, Proxy};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

pub struct WindowState {
    component: ComponentState,
    rendering: RenderingState<FemtoVGWindow>,
    event_context: EventContext,
    display_metrics: SharedDisplayMetrics,
    #[allow(dead_code)]
    pointer: ManagedWlPointer,
    active_popup_key: RefCell<Option<usize>>,
    main_surface: Rc<WlSurface>,
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

        let component =
            ComponentState::new(component_definition, builder.compilation_result, &window)?;

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
        let pointer = ManagedWlPointer::new(pointer_rc, Rc::clone(&connection));

        let has_fractional_scale = fractional_scale.is_some();
        let size = builder.size.unwrap_or_default();

        let main_surface_id = (*surface_rc).id();

        let display_metrics = Rc::new(RefCell::new(
            DisplayMetrics::new(builder.scale_factor, has_fractional_scale)
                .with_output_size(builder.output_size.unwrap_or_default()),
        ));

        let event_context = EventContext::new(
            Rc::clone(&window),
            main_surface_id,
            Rc::clone(&display_metrics),
        );

        let rendering = RenderingState::new(WindowRendererParams {
            window: Rc::clone(&window),
            surface,
            layer_surface,
            viewport,
            fractional_scale,
            height: builder.height,
            exclusive_zone: builder.exclusive_zone,
            size,
        });

        Ok(Self {
            component,
            rendering,
            event_context,
            display_metrics,
            pointer,
            active_popup_key: RefCell::new(None),
            main_surface: surface_rc,
        })
    }

    pub fn update_size(&mut self, width: u32, height: u32) {
        let scale_factor = self.event_context.scale_factor();
        self.rendering.update_size(width, height, scale_factor);
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        self.event_context
            .set_current_pointer_position(physical_x, physical_y);
    }

    pub fn size(&self) -> PhysicalSize {
        self.rendering.size()
    }

    pub fn current_pointer_position(&self) -> LogicalPosition {
        self.event_context.current_pointer_position()
    }

    pub(crate) fn window(&self) -> Rc<FemtoVGWindow> {
        Rc::clone(self.rendering.window())
    }

    pub(crate) fn layer_surface(&self) -> Rc<ZwlrLayerSurfaceV1> {
        self.rendering.layer_surface()
    }

    pub fn height(&self) -> u32 {
        self.rendering.height()
    }

    pub fn set_output_size(&mut self, output_size: PhysicalSize) {
        self.display_metrics
            .borrow_mut()
            .update_output_size(output_size);
        self.event_context.update_output_size(output_size);
    }

    pub fn output_size(&self) -> PhysicalSize {
        self.display_metrics.borrow().output_size()
    }

    pub const fn component_instance(&self) -> &ComponentInstance {
        self.component.component_instance()
    }

    #[must_use]
    pub fn compilation_result(&self) -> Option<Rc<CompilationResult>> {
        self.component.compilation_result()
    }

    pub fn render_frame_if_dirty(&self) -> Result<()> {
        self.rendering.render_frame_if_dirty()
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_factor(&mut self, scale_120ths: u32) {
        self.event_context.update_scale_factor(scale_120ths);

        let current_logical_size = self.rendering.logical_size();
        if current_logical_size.width > 0 && current_logical_size.height > 0 {
            self.update_size(current_logical_size.width, current_logical_size.height);
        }
    }

    pub fn scale_factor(&self) -> f32 {
        self.event_context.scale_factor()
    }

    pub const fn display_metrics(&self) -> &SharedDisplayMetrics {
        &self.display_metrics
    }

    pub fn last_pointer_serial(&self) -> u32 {
        self.event_context.last_pointer_serial()
    }

    pub fn set_last_pointer_serial(&mut self, serial: u32) {
        self.event_context.set_last_pointer_serial(serial);
    }

    pub fn set_shared_pointer_serial(&mut self, shared_serial: Rc<SharedPointerSerial>) {
        self.event_context.set_shared_pointer_serial(shared_serial);
    }

    pub fn set_popup_manager(&mut self, popup_manager: Rc<PopupManager>) {
        self.event_context.set_popup_manager(popup_manager);
    }

    pub fn set_entered_surface(&self, surface: &WlSurface) {
        if let Some(popup_manager) = self.event_context.popup_manager() {
            if let Some(popup_key) = popup_manager.find_popup_key_by_surface_id(&surface.id()) {
                *self.active_popup_key.borrow_mut() = Some(popup_key);
                return;
            }
        }
        *self.active_popup_key.borrow_mut() = None;
    }

    pub fn clear_entered_surface(&self) {
        *self.active_popup_key.borrow_mut() = None;
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent) {
        let active_popup = *self.active_popup_key.borrow();

        if let Some(popup_key) = active_popup {
            if let Some(popup_manager) = self.event_context.popup_manager() {
                if let Some(popup_window) = popup_manager.get_popup_window(popup_key) {
                    popup_window.dispatch_event(event);
                    return;
                }
            }
        }

        self.event_context
            .dispatch_to_active_window(event, &self.main_surface);
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_for_fractional_scale_object(
        &mut self,
        fractional_scale_proxy: &WpFractionalScaleV1,
        scale_120ths: u32,
    ) {
        let fractional_scale_id = fractional_scale_proxy.id();

        if let Some(main_fractional_scale) = self.rendering.fractional_scale() {
            if (**main_fractional_scale.inner()).id() == fractional_scale_id {
                self.update_scale_factor(scale_120ths);
                return;
            }
        }

        self.event_context
            .update_scale_for_fractional_scale_object(fractional_scale_proxy, scale_120ths);
    }

    pub fn popup_manager(&self) -> Option<&Rc<PopupManager>> {
        self.event_context.popup_manager()
    }
}

impl RuntimeStatePort for WindowState {
    fn render_frame_if_dirty(&self) -> CoreResult<(), DomainError> {
        WindowState::render_frame_if_dirty(self).map_err(|e| DomainError::Adapter {
            source: Box::new(e),
        })
    }
}
