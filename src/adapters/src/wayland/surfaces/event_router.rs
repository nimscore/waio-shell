use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::wayland::surfaces::popup_manager::{ActiveWindow, PopupManager};
use slint::platform::{WindowAdapter, WindowEvent};
use std::rc::Rc;
use wayland_client::{backend::ObjectId, protocol::wl_surface::WlSurface};

pub struct EventRouter {
    main_window: Rc<FemtoVGWindow>,
    popup_manager: Option<Rc<PopupManager>>,
    main_surface_id: ObjectId,
}

impl EventRouter {
    #[must_use]
    pub const fn new(main_window: Rc<FemtoVGWindow>, main_surface_id: ObjectId) -> Self {
        Self {
            main_window,
            popup_manager: None,
            main_surface_id,
        }
    }

    pub fn set_popup_manager(&mut self, popup_manager: Rc<PopupManager>) {
        self.popup_manager = Some(popup_manager);
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent, surface: &WlSurface) {
        if let Some(popup_manager) = &self.popup_manager {
            match popup_manager.get_active_window(surface, &self.main_surface_id) {
                ActiveWindow::Main => {
                    self.main_window.window().dispatch_event(event);
                }
                ActiveWindow::Popup(index) => {
                    if let Some(popup_window) = popup_manager.get_popup_window(index) {
                        popup_window.dispatch_event(event);
                    }
                }
                ActiveWindow::None => {}
            }
        }
    }
}
