use super::renderable_window::{RenderState, RenderableWindow};
use crate::errors::{RenderingError, Result};
use crate::wayland::surfaces::popup_manager::{OnCloseCallback, PopupId};
use core::ops::Deref;
use log::info;
use slint::{
    PhysicalSize, Window, WindowSize,
    platform::{Renderer, WindowAdapter, WindowEvent, femtovg_renderer::FemtoVGRenderer},
};
use slint_interpreter::ComponentInstance;
use std::cell::{Cell, OnceCell};
use std::rc::{Rc, Weak};

pub struct PopupWindow {
    window: Window,
    renderer: FemtoVGRenderer,
    render_state: Cell<RenderState>,
    size: Cell<PhysicalSize>,
    scale_factor: Cell<f32>,
    popup_id: Cell<Option<PopupId>>,
    on_close: OnceCell<OnCloseCallback>,
    configured: Cell<bool>,
    component_instance: OnceCell<ComponentInstance>,
}

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
                popup_id: Cell::new(None),
                on_close: OnceCell::new(),
                configured: Cell::new(false),
                component_instance: OnceCell::new(),
            }
        })
    }

    #[must_use]
    pub fn new_with_callback(renderer: FemtoVGRenderer, on_close: OnCloseCallback) -> Rc<Self> {
        let window = Self::new(renderer);
        window.on_close.set(on_close).ok();
        window
    }

    pub fn set_popup_id(&self, id: PopupId) {
        self.popup_id.set(Some(id));
    }

    pub fn close_popup(&self) {
        info!("Closing popup window - cleaning up resources");

        if let Err(e) = self.window.hide() {
            info!("Failed to hide popup window: {e}");
        }

        if let Some(id) = self.popup_id.get() {
            info!("Destroying popup with id {:?}", id);
            if let Some(on_close) = self.on_close.get() {
                on_close(id);
            }
        }

        self.popup_id.set(None);

        info!("Popup window cleanup complete");
    }

    pub fn popup_key(&self) -> Option<usize> {
        self.popup_id.get().map(PopupId::key)
    }

    pub fn mark_configured(&self) {
        info!("Popup window marked as configured");
        self.configured.set(true);
    }

    pub fn is_configured(&self) -> bool {
        self.configured.get()
    }

    pub fn set_component_instance(&self, instance: ComponentInstance) {
        info!("Setting component instance for popup window");
        if self.component_instance.set(instance).is_err() {
            info!("Component instance already set for popup window");
        }
    }

    pub fn request_resize(&self, width: f32, height: f32) {
        info!("Requesting popup resize to {}x{}", width, height);
        self.set_size(WindowSize::Logical(slint::LogicalSize::new(width, height)));
        RenderableWindow::request_redraw(self);
    }
}

impl RenderableWindow for PopupWindow {
    fn render_frame_if_dirty(&self) -> Result<()> {
        if !self.configured.get() {
            info!("Popup not yet configured, skipping render");
            return Ok(());
        }

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

    fn set_scale_factor(&self, scale_factor: f32) {
        info!("Setting popup scale factor to {scale_factor}");
        self.scale_factor.set(scale_factor);
        self.window()
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });
    }

    fn scale_factor(&self) -> f32 {
        self.scale_factor.get()
    }

    fn render_state(&self) -> &Cell<RenderState> {
        &self.render_state
    }

    fn size_cell(&self) -> &Cell<PhysicalSize> {
        &self.size
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
        self.size_impl()
    }

    fn set_size(&self, size: WindowSize) {
        self.set_size_impl(size);
    }

    fn request_redraw(&self) {
        RenderableWindow::request_redraw(self);
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
