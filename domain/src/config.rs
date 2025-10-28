#![allow(clippy::pub_use)]

pub use crate::entities::component::UiComponentHandle;
pub use crate::value_objects::anchor::AnchorEdges;
pub use crate::value_objects::dimensions::WindowHeight;
pub use crate::value_objects::layer::Layer;
pub use crate::value_objects::margins::Margins;

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub height: u32,
    pub margin: Margins,
    pub exclusive_zone: i32,
    pub scale_factor: f32,
    pub namespace: String,
    pub layer: Layer,
    pub anchor: AnchorEdges,
}

impl WindowConfig {
    #[must_use]
    pub fn new() -> Self {
        Self {
            height: 30,
            margin: Margins::default(),
            exclusive_zone: -1,
            namespace: "layer-shika".to_owned(),
            scale_factor: 1.0,
            layer: Layer::default(),
            anchor: AnchorEdges::default(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new()
    }
}
