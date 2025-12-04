use layer_shika_adapters::WindowState;
use layer_shika_adapters::platform::slint_interpreter::ComponentInstance;
use layer_shika_adapters::platform::wayland::{Anchor, WaylandKeyboardInteractivity, WaylandLayer};
use layer_shika_domain::value_objects::keyboard_interactivity::KeyboardInteractivity;
use layer_shika_domain::value_objects::layer::Layer;
use layer_shika_domain::value_objects::margins::Margins;

pub struct LayerSurfaceHandle<'a> {
    window_state: &'a WindowState,
}

impl<'a> LayerSurfaceHandle<'a> {
    pub(crate) fn from_window_state(window_state: &'a WindowState) -> Self {
        Self { window_state }
    }

    pub fn set_anchor(&self, anchor: Anchor) {
        self.window_state.layer_surface().set_anchor(anchor);
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.window_state.layer_surface().set_size(width, height);
    }

    pub fn set_exclusive_zone(&self, zone: i32) {
        self.window_state.layer_surface().set_exclusive_zone(zone);
    }

    pub fn set_margins(&self, margins: Margins) {
        self.window_state.layer_surface().set_margin(
            margins.top,
            margins.right,
            margins.bottom,
            margins.left,
        );
    }

    pub fn set_keyboard_interactivity(&self, mode: KeyboardInteractivity) {
        let wayland_mode = match mode {
            KeyboardInteractivity::None => WaylandKeyboardInteractivity::None,
            KeyboardInteractivity::Exclusive => WaylandKeyboardInteractivity::Exclusive,
            KeyboardInteractivity::OnDemand => WaylandKeyboardInteractivity::OnDemand,
        };
        self.window_state
            .layer_surface()
            .set_keyboard_interactivity(wayland_mode);
    }

    pub fn set_layer(&self, layer: Layer) {
        let wayland_layer = match layer {
            Layer::Background => WaylandLayer::Background,
            Layer::Bottom => WaylandLayer::Bottom,
            Layer::Top => WaylandLayer::Top,
            Layer::Overlay => WaylandLayer::Overlay,
        };
        self.window_state.layer_surface().set_layer(wayland_layer);
    }

    pub fn commit(&self) {
        self.window_state.commit_surface();
    }
}

pub trait ShellWindowConfigHandler {
    fn configure_window(&self, instance: &ComponentInstance, surface: LayerSurfaceHandle<'_>);
}

impl<F> ShellWindowConfigHandler for F
where
    F: Fn(&ComponentInstance, LayerSurfaceHandle<'_>),
{
    fn configure_window(&self, instance: &ComponentInstance, surface: LayerSurfaceHandle<'_>) {
        self(instance, surface);
    }
}

#[derive(Debug, Clone)]
pub struct ShellWindowHandle {
    pub name: String,
}
