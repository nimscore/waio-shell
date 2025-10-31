use crate::errors::{LayerShikaError, Result};
use crate::rendering::egl::context::EGLContext;
use crate::rendering::femtovg::popup_window::PopupWindow;
use crate::rendering::slint_integration::platform::{
    clear_popup_position_override, get_popup_position_override,
};
use log::info;
use slab::Slab;
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

use super::popup_surface::PopupSurface;
use super::surface_state::WindowState;

pub struct PopupContext {
    compositor: WlCompositor,
    xdg_wm_base: Option<XdgWmBase>,
    seat: WlSeat,
    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    viewporter: Option<WpViewporter>,
    display: WlDisplay,
}

impl PopupContext {
    #[must_use]
    pub fn new(
        compositor: WlCompositor,
        xdg_wm_base: Option<XdgWmBase>,
        seat: WlSeat,
        fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
        viewporter: Option<WpViewporter>,
        display: WlDisplay,
        _connection: Rc<Connection>,
    ) -> Self {
        Self {
            compositor,
            xdg_wm_base,
            seat,
            fractional_scale_manager,
            viewporter,
            display,
        }
    }
}

struct ActivePopup {
    surface: PopupSurface,
    window: Rc<PopupWindow>,
}

pub struct PopupManager {
    context: PopupContext,
    popups: RefCell<Slab<ActivePopup>>,
    current_scale_factor: RefCell<f32>,
    current_output_size: RefCell<PhysicalSize>,
}

impl PopupManager {
    #[must_use]
    pub const fn new(context: PopupContext, initial_scale_factor: f32) -> Self {
        Self {
            context,
            popups: RefCell::new(Slab::new()),
            current_scale_factor: RefCell::new(initial_scale_factor),
            current_output_size: RefCell::new(PhysicalSize::new(0, 0)),
        }
    }

    pub fn update_scale_factor(&self, scale_factor: f32) {
        *self.current_scale_factor.borrow_mut() = scale_factor;
    }

    pub fn update_output_size(&self, output_size: PhysicalSize) {
        *self.current_output_size.borrow_mut() = output_size;
    }

    pub fn create_popup(
        self: &Rc<Self>,
        queue_handle: &QueueHandle<WindowState>,
        parent_layer_surface: &ZwlrLayerSurfaceV1,
        last_pointer_serial: u32,
    ) -> Result<Rc<PopupWindow>> {
        let xdg_wm_base = self.context.xdg_wm_base.as_ref().ok_or_else(|| {
            LayerShikaError::WindowConfiguration {
                message: "xdg-shell not available for popups".into(),
            }
        })?;

        let pointer_position = if let Some((x, y)) = get_popup_position_override() {
            info!("Using explicit popup position: ({}, {})", x, y);
            clear_popup_position_override();
            slint::LogicalPosition::new(x, y)
        } else {
            log::error!("No popup position provided - using (0, 0) as fallback");
            slint::LogicalPosition::new(0.0, 0.0)
        };

        let scale_factor = *self.current_scale_factor.borrow();
        let output_size = *self.current_output_size.borrow();
        info!(
            "Creating popup window with scale factor {scale_factor} and output size {output_size:?}"
        );

        #[allow(clippy::cast_precision_loss)]
        let logical_size = slint::LogicalSize::new(
            output_size.width as f32 / scale_factor,
            output_size.height as f32 / scale_factor,
        );
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        let popup_size = PhysicalSize::new(
            (logical_size.width * scale_factor) as u32,
            (logical_size.height * scale_factor) as u32,
        );

        info!("Popup logical size: {logical_size:?}, physical size: {popup_size:?}");

        let popup_surface = PopupSurface::create(&super::popup_surface::PopupSurfaceParams {
            compositor: &self.context.compositor,
            xdg_wm_base,
            parent_layer_surface,
            fractional_scale_manager: self.context.fractional_scale_manager.as_ref(),
            viewporter: self.context.viewporter.as_ref(),
            queue_handle,
            position: pointer_position,
            size: popup_size,
            scale_factor,
        });

        popup_surface.grab(&self.context.seat, last_pointer_serial);

        let context = EGLContext::builder()
            .with_display_id(self.context.display.id())
            .with_surface_id(popup_surface.surface.id())
            .with_size(popup_size)
            .build()?;

        let renderer = FemtoVGRenderer::new(context)
            .map_err(|e| LayerShikaError::FemtoVGRendererCreation { source: e })?;

        let popup_window = PopupWindow::new(renderer);
        popup_window.set_scale_factor(scale_factor);
        popup_window.set_size(WindowSize::Logical(logical_size));

        let key = self.popups.borrow_mut().insert(ActivePopup {
            surface: popup_surface,
            window: Rc::clone(&popup_window),
        });
        popup_window.set_popup_manager(Rc::downgrade(self), key);

        info!("Popup window created successfully with key {key}");

        Ok(popup_window)
    }

    pub fn render_popups(&self) -> Result<()> {
        for (_key, popup) in self.popups.borrow().iter() {
            popup.window.render_frame_if_dirty()?;
        }
        Ok(())
    }

    pub const fn has_xdg_shell(&self) -> bool {
        self.context.xdg_wm_base.is_some()
    }

    pub fn mark_all_popups_dirty(&self) {
        for (_key, popup) in self.popups.borrow().iter() {
            popup.window.request_redraw();
        }
    }

    pub fn find_popup_key_by_surface_id(&self, surface_id: &ObjectId) -> Option<usize> {
        self.popups
            .borrow()
            .iter()
            .find_map(|(key, popup)| (popup.surface.surface.id() == *surface_id).then_some(key))
    }

    pub fn find_popup_key_by_fractional_scale_id(
        &self,
        fractional_scale_id: &ObjectId,
    ) -> Option<usize> {
        self.popups.borrow().iter().find_map(|(key, popup)| {
            popup
                .surface
                .fractional_scale
                .as_ref()
                .filter(|fs| fs.id() == *fractional_scale_id)
                .map(|_| key)
        })
    }

    pub fn get_popup_window(&self, key: usize) -> Option<Rc<PopupWindow>> {
        self.popups
            .borrow()
            .get(key)
            .map(|popup| Rc::clone(&popup.window))
    }

    pub fn destroy_popup(&self, key: usize) {
        if let Some(popup) = self.popups.borrow_mut().try_remove(key) {
            info!("Destroying popup with key {key}");
            popup.surface.destroy();
        }
    }

    pub fn find_popup_key_by_xdg_popup_id(&self, xdg_popup_id: &ObjectId) -> Option<usize> {
        self.popups
            .borrow()
            .iter()
            .find_map(|(key, popup)| (popup.surface.xdg_popup.id() == *xdg_popup_id).then_some(key))
    }
}
