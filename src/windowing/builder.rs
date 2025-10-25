use slint_interpreter::ComponentDefinition;
use smithay_client_toolkit::reexports::protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self},
    zwlr_layer_surface_v1::{Anchor, KeyboardInteractivity},
};

use crate::errors::{LayerShikaError, Result};

use super::{config::{Margins, WindowConfig}, WindowingSystem};

pub struct WindowingSystemBuilder {
    config: Option<WindowConfig>,
}

impl Default for WindowingSystemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowingSystemBuilder {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            config: None,
        }
    }

    #[must_use]
    pub const fn with_height(mut self, height: u32) -> Self {
        if let Some(ref mut config) = self.config {
            config.height = height;
        }
        self
    }

    #[must_use]
    pub const fn with_layer(mut self, layer: zwlr_layer_shell_v1::Layer) -> Self {
        if let Some(ref mut config) = self.config {
            config.layer = layer;
        }
        self
    }

    #[must_use]
    pub const fn with_margin(mut self, top: i32, right: i32, bottom: i32, left: i32) -> Self {
        if let Some(ref mut config) = self.config {
            config.margin = Margins { top, right, bottom, left };
        }
        self
    }

    #[must_use]
    pub const fn with_anchor(mut self, anchor: Anchor) -> Self {
        if let Some(ref mut config) = self.config {
            config.anchor = anchor;
        }
        self
    }

    #[must_use]
    pub const fn with_keyboard_interactivity(
        mut self,
        interactivity: KeyboardInteractivity,
    ) -> Self {
        if let Some(ref mut config) = self.config {
            config.keyboard_interactivity = interactivity;
        }
        self
    }

    #[must_use]
    pub const fn with_exclusive_zone(mut self, zone: i32) -> Self {
        if let Some(ref mut config) = self.config {
            config.exclusive_zone = zone;
        }
        self
    }

    #[must_use]
    pub fn with_namespace(mut self, namespace: String) -> Self {
        if let Some(ref mut config) = self.config {
            config.namespace = namespace;
        }
        self
    }

    #[must_use]
    pub const fn with_scale_factor(mut self, scale_factor: f32) -> Self {
        if let Some(ref mut config) = self.config {
            config.scale_factor = scale_factor;
        }
        self
    }

    #[must_use]
    pub fn with_component_definition(mut self, component: ComponentDefinition) -> Self {
        self.config = Some(WindowConfig::new(component));
        self
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn build(self) -> Result<WindowingSystem> {
        let config = self.config.as_ref().ok_or_else(|| {
            LayerShikaError::InvalidInput("Slint component not set".into())
        })?;
        WindowingSystem::new(config)
    }
}
