use crate::rendering::femtovg::main_window::FemtoVGWindow;
use crate::wayland::surfaces::display_metrics::SharedDisplayMetrics;
use crate::wayland::surfaces::popup_manager::{ActiveWindow, PopupManager};
use slint::platform::{WindowAdapter, WindowEvent};
use slint::{LogicalPosition, PhysicalSize};
use std::cell::Cell;
use std::rc::Rc;
use wayland_client::{
    Proxy,
    backend::ObjectId,
    protocol::{wl_pointer, wl_surface::WlSurface},
};
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
    popup_manager: Option<Rc<PopupManager>>,
    display_metrics: SharedDisplayMetrics,
    current_pointer_position: LogicalPosition,
    last_pointer_serial: u32,
    shared_pointer_serial: Option<Rc<SharedPointerSerial>>,
    active_surface: ActiveWindow,
    accumulated_axis_x: f32,
    accumulated_axis_y: f32,
    axis_source: Option<wl_pointer::AxisSource>,
}

impl EventContext {
    #[must_use]
    pub fn new(
        main_window: Rc<FemtoVGWindow>,
        main_surface_id: ObjectId,
        display_metrics: SharedDisplayMetrics,
    ) -> Self {
        Self {
            main_window,
            main_surface_id,
            popup_manager: None,
            display_metrics,
            current_pointer_position: LogicalPosition::new(0.0, 0.0),
            last_pointer_serial: 0,
            shared_pointer_serial: None,
            active_surface: ActiveWindow::None,
            accumulated_axis_x: 0.0,
            accumulated_axis_y: 0.0,
            axis_source: None,
        }
    }

    pub fn set_popup_manager(&mut self, popup_manager: Rc<PopupManager>) {
        self.popup_manager = Some(popup_manager);
    }

    pub const fn popup_manager(&self) -> Option<&Rc<PopupManager>> {
        self.popup_manager.as_ref()
    }

    #[must_use]
    pub fn scale_factor(&self) -> f32 {
        self.display_metrics.borrow().scale_factor()
    }

    pub fn update_scale_factor(&mut self, scale_120ths: u32) -> f32 {
        let new_scale_factor = self
            .display_metrics
            .borrow_mut()
            .update_scale_factor(scale_120ths);

        if let Some(popup_manager) = &self.popup_manager {
            popup_manager.update_scale_factor(new_scale_factor);
        }

        new_scale_factor
    }

    pub const fn current_pointer_position(&self) -> LogicalPosition {
        self.current_pointer_position
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn set_current_pointer_position(&mut self, physical_x: f64, physical_y: f64) {
        let has_fractional_scale = self.display_metrics.borrow().has_fractional_scale();
        let scale_factor = self.display_metrics.borrow().scale_factor();

        let logical_position = if has_fractional_scale {
            LogicalPosition::new(physical_x as f32, physical_y as f32)
        } else {
            LogicalPosition::new(
                (physical_x / f64::from(scale_factor)) as f32,
                (physical_y / f64::from(scale_factor)) as f32,
            )
        };
        self.current_pointer_position = logical_position;
    }

    pub const fn last_pointer_serial(&self) -> u32 {
        self.last_pointer_serial
    }

    pub fn set_last_pointer_serial(&mut self, serial: u32) {
        self.last_pointer_serial = serial;
        if let Some(ref shared_serial) = self.shared_pointer_serial {
            shared_serial.update(serial);
        }
    }

    pub fn set_shared_pointer_serial(&mut self, shared_serial: Rc<SharedPointerSerial>) {
        self.shared_pointer_serial = Some(shared_serial);
    }

    pub fn set_entered_surface(&mut self, surface: &WlSurface) {
        self.active_surface = if let Some(popup_manager) = &self.popup_manager {
            popup_manager.get_active_window(surface, &self.main_surface_id)
        } else {
            let surface_id = surface.id();
            if self.main_surface_id == surface_id {
                ActiveWindow::Main
            } else {
                ActiveWindow::None
            }
        };
    }

    pub fn clear_entered_surface(&mut self) {
        self.active_surface = ActiveWindow::None;
    }

    pub const fn is_popup_active(&self) -> bool {
        matches!(self.active_surface, ActiveWindow::Popup(_))
    }

    pub fn dispatch_to_active_window(&self, event: WindowEvent) {
        match self.active_surface {
            ActiveWindow::Main => {
                self.main_window.window().dispatch_event(event);
            }
            ActiveWindow::Popup(handle) => {
                let is_pointer_event = matches!(
                    event,
                    WindowEvent::PointerMoved { .. }
                        | WindowEvent::PointerPressed { .. }
                        | WindowEvent::PointerReleased { .. }
                        | WindowEvent::PointerScrolled { .. }
                );

                if let Some(popup_manager) = &self.popup_manager {
                    if let Some(popup_surface) = popup_manager.get_popup_window(handle.key()) {
                        popup_surface.dispatch_event(event);
                        if is_pointer_event {
                            popup_surface.request_redraw();
                        }
                    }
                }
            }
            ActiveWindow::None => {}
        }
    }

    pub fn dispatch_to_surface(&self, surface_id: &ObjectId, event: WindowEvent) {
        if self.main_surface_id == *surface_id {
            self.main_window.window().dispatch_event(event);
            return;
        }

        if let Some(popup_manager) = &self.popup_manager {
            if let Some(handle) = popup_manager.find_by_surface(surface_id) {
                if let Some(popup_surface) = popup_manager.get_popup_window(handle.key()) {
                    popup_surface.dispatch_event(event);
                    popup_surface.request_redraw();
                }
            }
        }
    }

    pub fn update_output_size(&self, output_size: PhysicalSize) {
        if let Some(popup_manager) = &self.popup_manager {
            popup_manager.update_output_size(output_size);
        }
    }

    pub fn update_scale_for_fractional_scale_object(
        &self,
        fractional_scale_proxy: &WpFractionalScaleV1,
        scale_120ths: u32,
    ) {
        if let Some(popup_manager) = &self.popup_manager {
            popup_manager
                .update_scale_for_fractional_scale_object(fractional_scale_proxy, scale_120ths);
        }
    }

    pub fn set_axis_source(&mut self, axis_source: wl_pointer::AxisSource) {
        self.axis_source = Some(axis_source);
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn accumulate_axis(&mut self, axis: wl_pointer::Axis, value: f64) {
        match axis {
            wl_pointer::Axis::HorizontalScroll => {
                self.accumulated_axis_x += value as f32;
            }
            wl_pointer::Axis::VerticalScroll => {
                self.accumulated_axis_y += value as f32;
            }
            _ => {}
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn accumulate_axis_discrete(&mut self, axis: wl_pointer::Axis, discrete: i32) {
        let delta = discrete as f32 * 60.0;

        match axis {
            wl_pointer::Axis::HorizontalScroll => {
                self.accumulated_axis_x += delta;
            }
            wl_pointer::Axis::VerticalScroll => {
                self.accumulated_axis_y += delta;
            }
            _ => {}
        }
    }

    pub fn take_accumulated_axis(&mut self) -> (f32, f32) {
        let delta_x = self.accumulated_axis_x;
        let delta_y = self.accumulated_axis_y;

        self.accumulated_axis_x = 0.0;
        self.accumulated_axis_y = 0.0;
        self.axis_source = None;

        (delta_x, delta_y)
    }
}
