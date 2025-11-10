use log::info;
use slint::LogicalPosition;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SharedPointerSerial {
    serial: RefCell<u32>,
}

impl Default for SharedPointerSerial {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedPointerSerial {
    pub const fn new() -> Self {
        Self {
            serial: RefCell::new(0),
        }
    }

    pub fn update(&self, serial: u32) {
        *self.serial.borrow_mut() = serial;
    }

    pub fn get(&self) -> u32 {
        *self.serial.borrow()
    }
}

pub struct ScaleCoordinator {
    scale_factor: f32,
    current_pointer_position: LogicalPosition,
    last_pointer_serial: u32,
    shared_pointer_serial: Option<Rc<SharedPointerSerial>>,
    has_fractional_scale: bool,
}

impl ScaleCoordinator {
    #[must_use]
    pub const fn new(scale_factor: f32, has_fractional_scale: bool) -> Self {
        Self {
            scale_factor,
            current_pointer_position: LogicalPosition::new(0.0, 0.0),
            last_pointer_serial: 0,
            shared_pointer_serial: None,
            has_fractional_scale,
        }
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
        new_scale_factor
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
    }

    pub const fn current_pointer_position(&self) -> LogicalPosition {
        self.current_pointer_position
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
}
