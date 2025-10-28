use crate::system::WindowingSystem;
use crate::Result;
use layer_shika_adapters::platform::slint_interpreter::ComponentDefinition;
use layer_shika_domain::config::{AnchorEdges, Layer, Margins, WindowConfig};

pub struct NeedsComponent;
pub struct HasComponent {
    component_definition: ComponentDefinition,
}

pub struct LayerShika<State> {
    state: State,
    config: WindowConfig,
}

impl LayerShika<NeedsComponent> {
    #[must_use]
    pub fn new(component_definition: ComponentDefinition) -> LayerShika<HasComponent> {
        LayerShika {
            state: HasComponent {
                component_definition,
            },
            config: WindowConfig::default(),
        }
    }
}

impl LayerShika<HasComponent> {
    #[must_use]
    pub const fn with_height(mut self, height: u32) -> Self {
        self.config.height = height;
        self
    }

    #[must_use]
    pub const fn with_layer(mut self, layer: Layer) -> Self {
        self.config.layer = layer;
        self
    }

    #[must_use]
    pub const fn with_margin(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        self.config.margin = Margins {
            top,
            right,
            bottom,
            left,
        };
        self
    }

    #[must_use]
    pub const fn with_anchor(mut self, anchor: AnchorEdges) -> Self {
        self.config.anchor = anchor;
        self
    }

    #[must_use]
    pub const fn with_exclusive_zone(mut self, zone: i32) -> Self {
        self.config.exclusive_zone = zone;
        self
    }

    #[must_use]
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.config.namespace = namespace;
        self
    }

    #[must_use]
    pub const fn with_scale_factor(mut self, scale_factor: f32) -> Self {
        self.config.scale_factor = scale_factor;
        self
    }

    pub fn build(self) -> Result<WindowingSystem> {
        WindowingSystem::new(self.state.component_definition, self.config)
    }
}
