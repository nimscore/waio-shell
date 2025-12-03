use crate::dimensions::ScaleFactor;
use crate::value_objects::anchor::AnchorEdges;
use crate::value_objects::dimensions::WindowDimension;
use crate::value_objects::keyboard_interactivity::KeyboardInteractivity;
use crate::value_objects::layer::Layer;
use crate::value_objects::margins::Margins;
use crate::value_objects::output_policy::OutputPolicy;

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub dimensions: WindowDimension,
    pub margin: Margins,
    pub exclusive_zone: i32,
    pub scale_factor: ScaleFactor,
    pub namespace: String,
    pub layer: Layer,
    pub anchor: AnchorEdges,
    pub keyboard_interactivity: KeyboardInteractivity,
    pub output_policy: OutputPolicy,
}

impl WindowConfig {
    #[must_use]
    pub fn new() -> Self {
        Self {
            dimensions: WindowDimension::default(),
            margin: Margins::default(),
            exclusive_zone: -1,
            namespace: "layer-shika".to_owned(),
            scale_factor: ScaleFactor::default(),
            layer: Layer::default(),
            anchor: AnchorEdges::default(),
            keyboard_interactivity: KeyboardInteractivity::default(),
            output_policy: OutputPolicy::default(),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self::new()
    }
}
