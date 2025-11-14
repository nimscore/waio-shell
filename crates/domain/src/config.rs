use crate::dimensions::ScaleFactor;
use crate::value_objects::anchor::AnchorEdges;
use crate::value_objects::dimensions::WindowHeight;
use crate::value_objects::keyboard_interactivity::KeyboardInteractivity;
use crate::value_objects::layer::Layer;
use crate::value_objects::margins::Margins;

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub height: WindowHeight,
    pub margin: Margins,
    pub exclusive_zone: i32,
    pub scale_factor: ScaleFactor,
    pub namespace: String,
    pub layer: Layer,
    pub anchor: AnchorEdges,
    pub keyboard_interactivity: KeyboardInteractivity,
}

impl WindowConfig {
    #[must_use]
    pub fn new() -> Self {
        Self {
            height: WindowHeight::default(),
            margin: Margins::default(),
            exclusive_zone: -1,
            namespace: "layer-shika".to_owned(),
            scale_factor: ScaleFactor::default(),
            layer: Layer::default(),
            anchor: AnchorEdges::default(),
            keyboard_interactivity: KeyboardInteractivity::default(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new()
    }
}
