use crate::wayland::managed_proxies::ManagedWlPointer;
use crate::wayland::services::popup_service::PopupService;
use crate::wayland::surfaces::event_bus::EventBus;
use crate::wayland::surfaces::event_router::EventRouter;
use crate::wayland::surfaces::scale_coordinator::{ScaleCoordinator, SharedPointerSerial};
use crate::wayland::surfaces::window_events::{ScaleSource, WindowStateEvent};
use slint::LogicalPosition;
use slint::platform::WindowEvent;
use std::rc::Rc;
use wayland_client::protocol::wl_surface::WlSurface;

pub struct InteractionState {
    #[allow(dead_code)]
    pointer: ManagedWlPointer,
    event_router: EventRouter,
    scale_coordinator: ScaleCoordinator,
    event_bus: EventBus,
}

impl InteractionState {
    #[must_use]
    pub fn new(
        pointer: ManagedWlPointer,
        event_router: EventRouter,
        scale_coordinator: ScaleCoordinator,
    ) -> Self {
        Self {
            pointer,
            event_router,
            scale_coordinator,
            event_bus: EventBus::new(),
        }
    }

    pub fn set_event_bus(&mut self, event_bus: EventBus) {
        self.event_bus = event_bus;
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        self.scale_coordinator
            .set_current_pointer_position(physical_x, physical_y);

        self.event_bus
            .publish(&WindowStateEvent::PointerPositionChanged {
                physical_x,
                physical_y,
            });
    }

    pub fn current_pointer_position(&self) -> LogicalPosition {
        self.scale_coordinator.current_pointer_position()
    }

    pub fn last_pointer_serial(&self) -> u32 {
        self.scale_coordinator.last_pointer_serial()
    }

    pub fn set_last_pointer_serial(&mut self, serial: u32) {
        self.scale_coordinator.set_last_pointer_serial(serial);

        self.event_bus
            .publish(&WindowStateEvent::PointerSerialUpdated { serial });
    }

    pub fn set_shared_pointer_serial(&mut self, shared_serial: Rc<SharedPointerSerial>) {
        self.scale_coordinator
            .set_shared_pointer_serial(shared_serial);
    }

    pub fn set_popup_service(&mut self, popup_service: Rc<PopupService>) {
        self.event_router.set_popup_service(popup_service);
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent, surface: &WlSurface) {
        self.event_router.dispatch_to_active_window(event, surface);
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_coordinator.scale_factor()
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_factor(&mut self, scale_120ths: u32) -> f32 {
        let new_scale = self.scale_coordinator.update_scale_factor(scale_120ths);

        self.event_bus
            .publish(&WindowStateEvent::ScaleFactorChanged {
                new_scale,
                source: ScaleSource::FractionalScale,
            });

        new_scale
    }
}
