use layer_shika_domain::prelude::{
    AnchorEdges, Layer, Margins, WindowConfig as DomainWindowConfig,
};
use slint_interpreter::ComponentDefinition;
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self},
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct LayerSurfaceParams {
    pub anchor: Anchor,
    pub margin: Margins,
    pub exclusive_zone: i32,
    pub keyboard_interactivity: KeyboardInteractivity,
    pub height: u32,
}

#[derive(Clone)]
pub struct WaylandWindowConfig {
    pub height: u32,
    pub layer: zwlr_layer_shell_v1::Layer,
    pub margin: Margins,
    pub anchor: Anchor,
    pub keyboard_interactivity: KeyboardInteractivity,
    pub exclusive_zone: i32,
    pub scale_factor: f32,
    pub namespace: String,
    pub component_definition: ComponentDefinition,
}

impl WaylandWindowConfig {
    #[must_use]
    pub fn from_domain_config(
        component_definition: ComponentDefinition,
        domain_config: DomainWindowConfig,
    ) -> Self {
        Self {
            height: domain_config.height,
            layer: convert_layer(domain_config.layer),
            margin: domain_config.margin,
            anchor: convert_anchor(domain_config.anchor),
            keyboard_interactivity: KeyboardInteractivity::OnDemand,
            exclusive_zone: domain_config.exclusive_zone,
            scale_factor: domain_config.scale_factor,
            namespace: domain_config.namespace,
            component_definition,
        }
    }
}

const fn convert_layer(layer: Layer) -> zwlr_layer_shell_v1::Layer {
    match layer {
        Layer::Background => zwlr_layer_shell_v1::Layer::Background,
        Layer::Bottom => zwlr_layer_shell_v1::Layer::Bottom,
        Layer::Top => zwlr_layer_shell_v1::Layer::Top,
        Layer::Overlay => zwlr_layer_shell_v1::Layer::Overlay,
    }
}

const fn convert_anchor(anchor: AnchorEdges) -> Anchor {
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
