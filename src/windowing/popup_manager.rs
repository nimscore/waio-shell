use crate::errors::{LayerShikaError, Result};
use crate::rendering::{egl_context::EGLContext, popup_window::PopupWindow};
use log::info;
use slint::{platform::femtovg_renderer::FemtoVGRenderer, PhysicalSize, WindowSize};
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::ZwlrLayerSurfaceV1;
use std::cell::RefCell;
use std::rc::Rc;
use wayland_client::{
    backend::ObjectId,
    protocol::{wl_compositor::WlCompositor, wl_display::WlDisplay, wl_seat::WlSeat},
    Connection, Proxy, QueueHandle,
};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1;
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

use super::{popup::PopupSurface, state::WindowState};

pub struct PopupContext {
    compositor: WlCompositor,
    xdg_wm_base: Option<XdgWmBase>,
    seat: WlSeat,
    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    viewporter: Option<WpViewporter>,
    display: WlDisplay,
    #[allow(dead_code)]
    connection: Rc<Connection>,
}

impl PopupContext {
    pub const fn new(
        compositor: WlCompositor,
        xdg_wm_base: Option<XdgWmBase>,
        seat: WlSeat,
        fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
        viewporter: Option<WpViewporter>,
        display: WlDisplay,
        connection: Rc<Connection>,
    ) -> Self {
        Self {
            compositor,
            xdg_wm_base,
            seat,
            fractional_scale_manager,
            viewporter,
            display,
            connection,
        }
    }
}

struct ActivePopup {
    surface: PopupSurface,
    window: Rc<PopupWindow>,
}

pub struct PopupManager {
    context: PopupContext,
    popups: RefCell<Vec<ActivePopup>>,
    current_scale_factor: RefCell<f32>,
}

impl PopupManager {
    pub const fn new(context: PopupContext, initial_scale_factor: f32) -> Self {
        Self {
            context,
            popups: RefCell::new(Vec::new()),
            current_scale_factor: RefCell::new(initial_scale_factor),
        }
    }

    pub fn update_scale_factor(&self, scale_factor: f32) {
        *self.current_scale_factor.borrow_mut() = scale_factor;
    }

    pub fn create_popup(
        &self,
        queue_handle: &QueueHandle<WindowState>,
        parent_layer_surface: &ZwlrLayerSurfaceV1,
        last_pointer_serial: u32,
    ) -> Result<Rc<PopupWindow>> {
        let xdg_wm_base = self.context.xdg_wm_base.as_ref().ok_or_else(|| {
            LayerShikaError::WaylandProtocol("xdg-shell not available for popups".into())
        })?;

        let scale_factor = *self.current_scale_factor.borrow();
        info!("Creating popup window with scale factor {scale_factor}");

        let logical_size = slint::LogicalSize::new(360.0, 524.0);
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        let popup_size = PhysicalSize::new(
            (logical_size.width * scale_factor) as u32,
            (logical_size.height * scale_factor) as u32,
        );

        info!("Popup logical size: {logical_size:?}, physical size: {popup_size:?}");

        let popup_surface = PopupSurface::create(&super::popup::PopupSurfaceParams {
            compositor: &self.context.compositor,
            xdg_wm_base,
            parent_layer_surface,
            fractional_scale_manager: self.context.fractional_scale_manager.as_ref(),
            viewporter: self.context.viewporter.as_ref(),
            queue_handle,
            position: slint::LogicalPosition::new(0.0, 0.0),
            size: popup_size,
            scale_factor,
        });

        popup_surface.grab(&self.context.seat, last_pointer_serial);

        let context = EGLContext::builder()
            .with_display_id(self.context.display.id())
            .with_surface_id(popup_surface.surface.id())
            .with_size(popup_size)
            .build()
            .map_err(|e| LayerShikaError::EGLContextCreation(e.to_string()))?;

        let renderer = FemtoVGRenderer::new(context)
            .map_err(|e| LayerShikaError::FemtoVGRendererCreation(e.to_string()))?;

        let popup_window = PopupWindow::new(renderer);
        popup_window.set_scale_factor(scale_factor);
        popup_window.set_size(WindowSize::Logical(logical_size));

        info!("Popup window created successfully");

        self.popups.borrow_mut().push(ActivePopup {
            surface: popup_surface,
            window: Rc::clone(&popup_window),
        });

        Ok(popup_window)
    }

    pub fn render_popups(&self) -> Result<()> {
        for popup in self.popups.borrow().iter() {
            popup.window.render_frame_if_dirty()?;
        }
        Ok(())
    }

    pub const fn has_xdg_shell(&self) -> bool {
        self.context.xdg_wm_base.is_some()
    }

    #[allow(dead_code)]
    pub fn popup_count(&self) -> usize {
        self.popups.borrow().len()
    }

    pub fn mark_all_popups_dirty(&self) {
        for popup in self.popups.borrow().iter() {
            popup.window.request_redraw();
        }
    }

    pub fn find_popup_index_by_surface_id(&self, surface_id: &ObjectId) -> Option<usize> {
        for (index, popup) in self.popups.borrow().iter().enumerate() {
            if popup.surface.surface.id() == *surface_id {
                return Some(index);
            }
        }
        None
    }

    pub fn find_popup_index_by_fractional_scale_id(
        &self,
        fractional_scale_id: &ObjectId,
    ) -> Option<usize> {
        for (index, popup) in self.popups.borrow().iter().enumerate() {
            if let Some(ref fs) = popup.surface.fractional_scale {
                if fs.id() == *fractional_scale_id {
                    return Some(index);
                }
            }
        }
        None
    }

    pub fn get_popup_window(&self, index: usize) -> Option<Rc<PopupWindow>> {
        self.popups
            .borrow()
            .get(index)
            .map(|popup| Rc::clone(&popup.window))
    }
}
