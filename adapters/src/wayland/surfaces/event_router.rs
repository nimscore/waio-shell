use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::wayland::services::popup_service::{ActiveWindow, PopupService};
use layer_shika_domain::value_objects::popup_request::PopupHandle;
use slint::platform::{WindowAdapter, WindowEvent};
use std::rc::Rc;
use wayland_client::{backend::ObjectId, protocol::wl_surface::WlSurface};

pub struct EventRouter {
    main_window: Rc<FemtoVGWindow>,
    popup_service: Option<Rc<PopupService>>,
    main_surface_id: ObjectId,
}

impl EventRouter {
    #[must_use]
    pub const fn new(main_window: Rc<FemtoVGWindow>, main_surface_id: ObjectId) -> Self {
        Self {
            main_window,
            popup_service: None,
            main_surface_id,
        }
    }

    pub fn set_popup_service(&mut self, popup_service: Rc<PopupService>) {
        self.popup_service = Some(popup_service);
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent, surface: &WlSurface) {
        if let Some(popup_service) = &self.popup_service {
            match popup_service.get_active_window(surface, &self.main_surface_id) {
                ActiveWindow::Main => {
                    self.main_window.window().dispatch_event(event);
                }
                ActiveWindow::Popup(index) => {
                    if let Some(popup_window) =
                        popup_service.get_popup_window(PopupHandle::new(index))
                    {
                        popup_window.dispatch_event(event);
                    }
                }
                ActiveWindow::None => {}
            }
        }
    }
}
