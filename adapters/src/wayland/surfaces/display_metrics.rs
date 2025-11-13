use log::info;
use slint::PhysicalSize;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub trait DisplayMetricsObserver {
    fn on_scale_factor_changed(&self, new_scale: f32);
    fn on_output_size_changed(&self, new_size: PhysicalSize);
}

pub struct DisplayMetrics {
    scale_factor: f32,
    output_size: PhysicalSize,
    surface_size: PhysicalSize,
    has_fractional_scale: bool,
    observers: RefCell<Vec<Weak<dyn DisplayMetricsObserver>>>,
}

impl DisplayMetrics {
    #[must_use]
    pub fn new(scale_factor: f32, has_fractional_scale: bool) -> Self {
        Self {
            scale_factor,
            output_size: PhysicalSize::new(0, 0),
            surface_size: PhysicalSize::new(0, 0),
            has_fractional_scale,
            observers: RefCell::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn with_output_size(mut self, output_size: PhysicalSize) -> Self {
        self.output_size = output_size;
        self
    }

    #[must_use]
    pub const fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    #[must_use]
    pub const fn output_size(&self) -> PhysicalSize {
        self.output_size
    }

    #[must_use]
    pub const fn surface_size(&self) -> PhysicalSize {
        self.surface_size
    }

    #[must_use]
    pub const fn has_fractional_scale(&self) -> bool {
        self.has_fractional_scale
    }

    pub fn register_observer(&self, observer: Weak<dyn DisplayMetricsObserver>) {
        self.observers.borrow_mut().push(observer);
        self.cleanup_dead_observers();
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update_scale_factor(&mut self, scale_120ths: u32) -> f32 {
        let new_scale_factor = scale_120ths as f32 / 120.0;
        let old_scale_factor = self.scale_factor;

        if (self.scale_factor - new_scale_factor).abs() > f32::EPSILON {
            info!(
                "DisplayMetrics: Updating scale factor from {} to {} ({}x)",
                old_scale_factor, new_scale_factor, scale_120ths
            );
            self.scale_factor = new_scale_factor;
            self.recalculate_surface_size();
            self.notify_scale_factor_changed(new_scale_factor);
        }

        new_scale_factor
    }

    pub fn update_output_size(&mut self, output_size: PhysicalSize) {
        if self.output_size != output_size {
            info!(
                "DisplayMetrics: Updating output size from {:?} to {:?}",
                self.output_size, output_size
            );
            self.output_size = output_size;
            self.recalculate_surface_size();
            self.notify_output_size_changed(output_size);
        }
    }

    pub fn update_surface_size(&mut self, surface_size: PhysicalSize) {
        self.surface_size = surface_size;
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn recalculate_surface_size(&mut self) {
        if self.output_size.width > 0 && self.output_size.height > 0 && self.scale_factor > 0.0 {
            self.surface_size = PhysicalSize::new(
                (self.output_size.width as f32 / self.scale_factor) as u32,
                (self.output_size.height as f32 / self.scale_factor) as u32,
            );
        }
    }

    fn notify_scale_factor_changed(&self, new_scale: f32) {
        self.observers.borrow_mut().retain(|observer| {
            if let Some(obs) = observer.upgrade() {
                obs.on_scale_factor_changed(new_scale);
                true
            } else {
                false
            }
        });
    }

    fn notify_output_size_changed(&self, new_size: PhysicalSize) {
        self.observers.borrow_mut().retain(|observer| {
            if let Some(obs) = observer.upgrade() {
                obs.on_output_size_changed(new_size);
                true
            } else {
                false
            }
        });
    }

    fn cleanup_dead_observers(&self) {
        self.observers
            .borrow_mut()
            .retain(|obs| obs.upgrade().is_some());
    }
}

pub type SharedDisplayMetrics = Rc<RefCell<DisplayMetrics>>;
