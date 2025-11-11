use crate::wayland::services::popup_service::PopupService;
use crate::wayland::surfaces::event_bus::EventBus;
use crate::wayland::surfaces::popup_manager::PopupManager;
use crate::wayland::surfaces::window_events::WindowStateEvent;
use slint::PhysicalSize;
use std::rc::Rc;
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

pub struct PopupState {
    popup_service: Option<Rc<PopupService>>,
    event_bus: EventBus,
}

impl Default for PopupState {
    fn default() -> Self {
        Self::new()
    }
}

impl PopupState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            popup_service: None,
            event_bus: EventBus::new(),
        }
    }

    pub fn set_event_bus(&mut self, event_bus: EventBus) {
        self.event_bus = event_bus;
    }

    pub fn set_popup_service(&mut self, popup_service: Rc<PopupService>) {
        self.popup_service = Some(popup_service);
        self.event_bus
            .publish(&WindowStateEvent::PopupConfigurationChanged);
    }

    pub fn set_popup_manager(&mut self, popup_manager: Rc<PopupManager>) {
        self.popup_service = Some(Rc::new(PopupService::new(popup_manager)));
        self.event_bus
            .publish(&WindowStateEvent::PopupConfigurationChanged);
    }

    pub fn update_output_size(&self, output_size: PhysicalSize) {
        if let Some(popup_service) = &self.popup_service {
            popup_service.update_output_size(output_size);
        }

        self.event_bus
            .publish(&WindowStateEvent::OutputSizeChanged { output_size });
    }

    pub fn update_scale_factor(&self, scale_factor: f32) {
        if let Some(popup_service) = &self.popup_service {
            popup_service.update_scale_factor(scale_factor);
        }
    }

    pub const fn popup_service(&self) -> &Option<Rc<PopupService>> {
        &self.popup_service
    }

    pub fn popup_manager(&self) -> Option<Rc<PopupManager>> {
        self.popup_service
            .as_ref()
            .map(|service| Rc::clone(service.manager()))
    }

    pub fn update_scale_for_fractional_scale_object(
        &self,
        fractional_scale_proxy: &WpFractionalScaleV1,
        scale_120ths: u32,
    ) {
        if let Some(popup_service) = &self.popup_service {
            popup_service
                .update_scale_for_fractional_scale_object(fractional_scale_proxy, scale_120ths);
        }
    }
}
