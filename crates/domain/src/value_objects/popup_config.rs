use crate::value_objects::handle::PopupHandle;
use crate::value_objects::output_target::OutputTarget;
use crate::value_objects::popup_behavior::PopupBehavior;
use crate::value_objects::popup_position::{Offset, PopupPosition};
use crate::value_objects::popup_size::PopupSize;

/// Declarative popup configuration (runtime modifiable)
#[derive(Debug, Clone)]
pub struct PopupConfig {
    /// Component name from compiled Slint file
    pub component: String,

    /// Positioning configuration
    pub position: PopupPosition,

    /// Size configuration
    pub size: PopupSize,

    /// Popup behavior flags
    pub behavior: PopupBehavior,

    /// Output targeting
    pub output: OutputTarget,

    /// Parent popup (for hierarchical popups)
    pub parent: Option<PopupHandle>,

    /// Z-order relative to siblings
    pub z_index: i32,

    /// Callback invoked by the component to request close
    pub close_callback: Option<String>,

    /// Callback invoked by the component to request resize (content-sizing)
    pub resize_callback: Option<String>,
}

impl PopupConfig {
    #[must_use]
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            position: PopupPosition::Cursor {
                offset: Offset::default(),
            },
            size: PopupSize::default(),
            behavior: PopupBehavior::default(),
            output: OutputTarget::Active,
            parent: None,
            z_index: 0,
            close_callback: None,
            resize_callback: None,
        }
    }
}
