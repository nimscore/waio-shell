use crate::errors::{RenderingError, Result};
use crate::wayland::surfaces::popup_manager::PopupManager;
use core::ops::Deref;
use log::info;
use slint::{
    ComponentHandle, PhysicalSize, Window, WindowSize,
    platform::{Renderer, WindowAdapter, WindowEvent, femtovg_renderer::FemtoVGRenderer},
};
use slint_interpreter::ComponentInstance;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use super::main_window::RenderState;

#[allow(dead_code)]
pub struct PopupWindow {
    window: Window,
    renderer: FemtoVGRenderer,
    render_state: Cell<RenderState>,
    size: Cell<PhysicalSize>,
    scale_factor: Cell<f32>,
    popup_manager: RefCell<Weak<PopupManager>>,
    popup_key: Cell<Option<usize>>,
    component_instance: RefCell<Option<ComponentInstance>>,
}

#[allow(dead_code)]
impl PopupWindow {
    #[must_use]
    pub fn new(renderer: FemtoVGRenderer) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| {
            let window = Window::new(Weak::clone(weak_self) as Weak<dyn WindowAdapter>);
            Self {
                window,
                renderer,
                render_state: Cell::new(RenderState::Clean),
                size: Cell::new(PhysicalSize::default()),
                scale_factor: Cell::new(1.),
                popup_manager: RefCell::new(Weak::new()),
                popup_key: Cell::new(None),
                component_instance: RefCell::new(None),
            }
        })
    }

    pub fn set_component_instance(&self, instance: ComponentInstance) {
        *self.component_instance.borrow_mut() = Some(instance);
    }

    pub fn set_popup_manager(&self, popup_manager: Weak<PopupManager>, key: usize) {
        *self.popup_manager.borrow_mut() = popup_manager;
        self.popup_key.set(Some(key));
    }

    pub fn close_popup(&self) {
        info!("Closing popup window - cleaning up resources");

        if let Some(instance) = self.component_instance.borrow_mut().take() {
            info!("Hiding ComponentInstance to release strong reference from show()");
            if let Err(e) = instance.hide() {
                info!("Failed to hide component instance: {e}");
            }
        }

        if let Err(e) = self.window.hide() {
            info!("Failed to hide popup window: {e}");
        }

        if let Some(popup_manager) = self.popup_manager.borrow().upgrade() {
            if let Some(key) = self.popup_key.get() {
                info!("Destroying popup with key {key}");
                popup_manager.destroy_popup(key);
            }
        }

        *self.popup_manager.borrow_mut() = Weak::new();
        self.popup_key.set(None);

        info!("Popup window cleanup complete");
    }

    pub fn render_frame_if_dirty(&self) -> Result<()> {
        if matches!(
            self.render_state.replace(RenderState::Clean),
            RenderState::Dirty
        ) {
            info!(
                "Rendering popup frame (size: {:?}, scale: {})",
                self.size.get(),
                self.scale_factor.get()
            );
            self.renderer
                .render()
                .map_err(|e| RenderingError::Operation {
                    message: format!("Error rendering popup frame: {e}"),
                })?;
            info!("Popup frame rendered successfully");
        }
        Ok(())
    }

    pub fn set_scale_factor(&self, scale_factor: f32) {
        info!("Setting popup scale factor to {scale_factor}");
        self.scale_factor.set(scale_factor);
        self.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor.get()
    }

    pub fn popup_key(&self) -> Option<usize> {
        self.popup_key.get()
    }
}

impl WindowAdapter for PopupWindow {
    fn window(&self) -> &Window {
        &self.window
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn size(&self) -> PhysicalSize {
        self.size.get()
    }

    fn set_size(&self, size: WindowSize) {
        self.size.set(size.to_physical(self.scale_factor()));
        self.window.dispatch_event(WindowEvent::Resized {
            size: size.to_logical(self.scale_factor()),
        });
    }

    fn request_redraw(&self) {
        self.render_state.set(RenderState::Dirty);
    }
}

impl Deref for PopupWindow {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl Drop for PopupWindow {
    fn drop(&mut self) {
        info!("PopupWindow being dropped - resources will be released");
    }
}
