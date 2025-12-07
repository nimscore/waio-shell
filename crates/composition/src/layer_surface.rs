use layer_shika_adapters::SurfaceState;
use layer_shika_adapters::platform::slint_interpreter::ComponentInstance;
use layer_shika_adapters::platform::wayland::{Anchor, WaylandKeyboardInteractivity, WaylandLayer};
use layer_shika_domain::value_objects::anchor::AnchorEdges;
use layer_shika_domain::value_objects::keyboard_interactivity::KeyboardInteractivity;
use layer_shika_domain::value_objects::layer::Layer;
use layer_shika_domain::value_objects::margins::Margins;

pub struct LayerSurfaceHandle<'a> {
    window_state: &'a SurfaceState,
}

impl<'a> LayerSurfaceHandle<'a> {
    pub(crate) fn from_window_state(window_state: &'a SurfaceState) -> Self {
        Self { window_state }
    }

    pub fn set_anchor(&self, anchor: Anchor) {
        self.window_state.layer_surface().set_anchor(anchor);
    }

    pub fn set_anchor_edges(&self, anchor: AnchorEdges) {
        let wayland_anchor = Self::convert_anchor(anchor);
        self.window_state.layer_surface().set_anchor(wayland_anchor);
    }

    fn convert_anchor(anchor: AnchorEdges) -> Anchor {
        let mut result = Anchor::empty();

        if anchor.has_top() {
            result = result.union(Anchor::Top);
        }
        if anchor.has_bottom() {
            result = result.union(Anchor::Bottom);
        }
        if anchor.has_left() {
            result = result.union(Anchor::Left);
        }
        if anchor.has_right() {
            result = result.union(Anchor::Right);
        }

        result
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

pub trait ShellSurfaceConfigHandler {
    fn configure_surface(&self, instance: &ComponentInstance, surface: LayerSurfaceHandle<'_>);
}

impl<F> ShellSurfaceConfigHandler for F
where
    F: Fn(&ComponentInstance, LayerSurfaceHandle<'_>),
{
    fn configure_surface(&self, instance: &ComponentInstance, surface: LayerSurfaceHandle<'_>) {
        self(instance, surface);
    }
}
