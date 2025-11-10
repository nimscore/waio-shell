use crate::errors::Result;
use crate::rendering::femtovg::popup_window::PopupWindow;
use layer_shika_domain::value_objects::popup_request::{PopupHandle, PopupRequest};
use log::info;
use slint::PhysicalSize;
use std::cell::Cell;
use std::rc::Rc;
use wayland_client::{Proxy, backend::ObjectId, protocol::wl_surface::WlSurface};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

use super::super::surfaces::popup_manager::PopupManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveWindow {
    Main,
    Popup(usize),
    None,
}

pub struct PopupService {
    manager: Rc<PopupManager>,
    scale_factor: Cell<f32>,
}

impl PopupService {
    #[must_use]
    pub fn new(manager: Rc<PopupManager>) -> Self {
        let scale_factor = manager.scale_factor();
        Self {
            manager,
            scale_factor: Cell::new(scale_factor),
        }
    }

    pub fn show(&self, request: PopupRequest, width: f32, height: f32) {
        self.manager.set_pending_popup(request, width, height);
    }

    pub fn close(&self, handle: PopupHandle) -> Result<()> {
        self.manager.destroy_popup(handle.key());
        Ok(())
    }

    pub fn close_current(&self) {
        self.manager.close_current_popup();
    }

    #[must_use]
    pub fn find_by_surface(&self, surface_id: &ObjectId) -> Option<PopupHandle> {
        self.manager
            .find_popup_key_by_surface_id(surface_id)
            .map(PopupHandle::new)
    }

    #[must_use]
    pub fn find_by_fractional_scale(&self, fractional_scale_id: &ObjectId) -> Option<PopupHandle> {
        self.manager
            .find_popup_key_by_fractional_scale_id(fractional_scale_id)
            .map(PopupHandle::new)
    }

    #[must_use]
    pub fn find_by_xdg_popup(&self, xdg_popup_id: &ObjectId) -> Option<PopupHandle> {
        self.manager
            .find_popup_key_by_xdg_popup_id(xdg_popup_id)
            .map(PopupHandle::new)
    }

    #[must_use]
    pub fn find_by_xdg_surface(&self, xdg_surface_id: &ObjectId) -> Option<PopupHandle> {
        self.manager
            .find_popup_key_by_xdg_surface_id(xdg_surface_id)
            .map(PopupHandle::new)
    }

    #[must_use]
    pub fn get_popup_window(&self, handle: PopupHandle) -> Option<Rc<PopupWindow>> {
        self.manager.get_popup_window(handle.key())
    }

    pub fn update_scale_factor(&self, scale_factor: f32) {
        self.scale_factor.set(scale_factor);
        self.manager.update_scale_factor(scale_factor);
        self.manager.mark_all_popups_dirty();
    }

    pub fn update_output_size(&self, output_size: PhysicalSize) {
        self.manager.update_output_size(output_size);
    }

    #[must_use]
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor.get()
    }

    #[must_use]
    pub fn get_active_window(
        &self,
        surface: &WlSurface,
        main_surface_id: &ObjectId,
    ) -> ActiveWindow {
        let surface_id = surface.id();

        if *main_surface_id == surface_id {
            return ActiveWindow::Main;
        }

        if let Some(popup_key) = self.manager.find_popup_key_by_surface_id(&surface_id) {
            return ActiveWindow::Popup(popup_key);
        }

        ActiveWindow::None
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_for_fractional_scale_object(
        &self,
        fractional_scale_proxy: &WpFractionalScaleV1,
        scale_120ths: u32,
    ) {
        let fractional_scale_id = fractional_scale_proxy.id();

        if let Some(popup_key) = self
            .manager
            .find_popup_key_by_fractional_scale_id(&fractional_scale_id)
        {
            if let Some(popup_window) = self.manager.get_popup_window(popup_key) {
                let new_scale_factor = scale_120ths as f32 / 120.0;
                info!("Updating popup scale factor to {new_scale_factor} ({scale_120ths}x)");
                popup_window.set_scale_factor(new_scale_factor);
                popup_window.request_redraw();
            }
        }
    }

    pub fn render_popups(&self) -> Result<()> {
        self.manager.render_popups()
    }

    pub fn mark_popup_configured(&self, handle: PopupHandle) {
        self.manager.mark_popup_configured(handle.key());
    }

    pub fn update_popup_viewport(
        &self,
        handle: PopupHandle,
        logical_width: i32,
        logical_height: i32,
    ) {
        self.manager
            .update_popup_viewport(handle.key(), logical_width, logical_height);
    }

    #[must_use]
    pub fn get_popup_info(&self, handle: PopupHandle) -> Option<(PopupRequest, u32)> {
        self.manager.get_popup_info(handle.key())
    }

    #[must_use]
    pub fn manager(&self) -> &Rc<PopupManager> {
        &self.manager
    }

    #[must_use]
    pub fn has_xdg_shell(&self) -> bool {
        self.manager.has_xdg_shell()
    }

    #[must_use]
    pub fn current_popup_key(&self) -> Option<usize> {
        self.manager.current_popup_key()
    }

    #[must_use]
    pub fn take_pending_popup(&self) -> Option<(PopupRequest, f32, f32)> {
        self.manager.take_pending_popup()
    }
}
