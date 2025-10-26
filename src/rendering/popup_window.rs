use crate::errors::{LayerShikaError, Result};
use core::ops::Deref;
use log::info;
use slint::{
    platform::{femtovg_renderer::FemtoVGRenderer, Renderer, WindowAdapter, WindowEvent},
    PhysicalSize, Window, WindowSize,
};
use std::cell::Cell;
use std::rc::{Rc, Weak};

use super::femtovg_window::RenderState;

#[allow(dead_code)]
pub struct PopupWindow {
    window: Window,
    renderer: FemtoVGRenderer,
    render_state: Cell<RenderState>,
    size: Cell<PhysicalSize>,
    scale_factor: Cell<f32>,
}

#[allow(dead_code)]
impl PopupWindow {
    pub fn new(renderer: FemtoVGRenderer) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| {
            let window = Window::new(Weak::clone(weak_self) as Weak<dyn WindowAdapter>);
            Self {
                window,
                renderer,
                render_state: Cell::new(RenderState::Clean),
                size: Cell::new(PhysicalSize::default()),
                scale_factor: Cell::new(1.),
            }
        })
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
            self.renderer.render().map_err(|e| {
                LayerShikaError::Rendering(format!("Error rendering popup frame: {e}"))
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

    pub const fn scale_factor(&self) -> f32 {
        self.scale_factor.get()
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
