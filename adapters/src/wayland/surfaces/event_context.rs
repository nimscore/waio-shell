use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::wayland::services::popup_service::{ActiveWindow, PopupService};
use crate::wayland::surfaces::event_bus::EventBus;
use crate::wayland::surfaces::popup_manager::PopupManager;
use crate::wayland::surfaces::window_events::{ScaleSource, WindowStateEvent};
use layer_shika_domain::value_objects::popup_request::PopupHandle;
use log::info;
use slint::platform::{WindowAdapter, WindowEvent};
use slint::{LogicalPosition, PhysicalSize};
use std::cell::Cell;
use std::rc::Rc;
use wayland_client::{backend::ObjectId, protocol::wl_surface::WlSurface};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1::WpFractionalScaleV1;

pub struct SharedPointerSerial {
    serial: Cell<u32>,
}

impl Default for SharedPointerSerial {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedPointerSerial {
    pub const fn new() -> Self {
        Self {
            serial: Cell::new(0),
        }
    }

    pub fn update(&self, serial: u32) {
        self.serial.set(serial);
    }

    pub fn get(&self) -> u32 {
        self.serial.get()
    }
}

pub struct EventContext {
    main_window: Rc<FemtoVGWindow>,
    main_surface_id: ObjectId,
    popup_service: Option<Rc<PopupService>>,
    event_bus: EventBus,
    scale_factor: f32,
    has_fractional_scale: bool,
    current_pointer_position: LogicalPosition,
    last_pointer_serial: u32,
    shared_pointer_serial: Option<Rc<SharedPointerSerial>>,
}

impl EventContext {
    #[must_use]
    pub fn new(
        main_window: Rc<FemtoVGWindow>,
        main_surface_id: ObjectId,
        scale_factor: f32,
        has_fractional_scale: bool,
    ) -> Self {
        Self {
            main_window,
            main_surface_id,
            popup_service: None,
            event_bus: EventBus::new(),
            scale_factor,
            has_fractional_scale,
            current_pointer_position: LogicalPosition::new(0.0, 0.0),
            last_pointer_serial: 0,
            shared_pointer_serial: None,
        }
    }

    pub fn set_event_bus(&mut self, event_bus: EventBus) {
        self.event_bus = event_bus;
    }

    pub const fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    pub fn set_popup_service(&mut self, popup_service: Rc<PopupService>) {
        self.popup_service = Some(popup_service);
        self.event_bus
            .publish(&WindowStateEvent::PopupConfigurationChanged);
    }

    pub const fn popup_service(&self) -> &Option<Rc<PopupService>> {
        &self.popup_service
    }

    pub fn popup_manager(&self) -> Option<Rc<PopupManager>> {
        self.popup_service
            .as_ref()
            .map(|service| Rc::clone(service.manager()))
    }

    pub const fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_factor(&mut self, scale_120ths: u32) -> f32 {
        let new_scale_factor = scale_120ths as f32 / 120.0;
        let old_scale_factor = self.scale_factor;
        info!(
            "Updating scale factor from {} to {} ({}x)",
            old_scale_factor, new_scale_factor, scale_120ths
        );
        self.scale_factor = new_scale_factor;

        if let Some(popup_service) = &self.popup_service {
            popup_service.update_scale_factor(new_scale_factor);
        }

        self.event_bus
            .publish(&WindowStateEvent::ScaleFactorChanged {
                new_scale: new_scale_factor,
                source: ScaleSource::FractionalScale,
            });

        new_scale_factor
    }

    pub const fn current_pointer_position(&self) -> LogicalPosition {
        self.current_pointer_position
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        let logical_position = if self.has_fractional_scale {
            LogicalPosition::new(physical_x as f32, physical_y as f32)
        } else {
            LogicalPosition::new(
                (physical_x / f64::from(self.scale_factor)) as f32,
                (physical_y / f64::from(self.scale_factor)) as f32,
            )
        };
        self.current_pointer_position = logical_position;

        self.event_bus
            .publish(&WindowStateEvent::PointerPositionChanged {
                physical_x,
                physical_y,
            });
    }

    pub const fn last_pointer_serial(&self) -> u32 {
        self.last_pointer_serial
    }

    pub fn set_last_pointer_serial(&mut self, serial: u32) {
        self.last_pointer_serial = serial;
        if let Some(ref shared_serial) = self.shared_pointer_serial {
            shared_serial.update(serial);
        }

        self.event_bus
            .publish(&WindowStateEvent::PointerSerialUpdated { serial });
    }

    pub fn set_shared_pointer_serial(&mut self, shared_serial: Rc<SharedPointerSerial>) {
        self.shared_pointer_serial = Some(shared_serial);
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

    pub fn update_output_size(&self, output_size: PhysicalSize) {
        if let Some(popup_service) = &self.popup_service {
            popup_service.update_output_size(output_size);
        }

        self.event_bus
            .publish(&WindowStateEvent::OutputSizeChanged { output_size });
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
