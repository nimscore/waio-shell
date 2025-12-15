use crate::popup::PopupShell;
use crate::{Error, Result};
use layer_shika_domain::dimensions::LogicalRect;
use layer_shika_domain::errors::DomainError;
use layer_shika_domain::value_objects::handle::PopupHandle;
use layer_shika_domain::value_objects::output_target::OutputTarget;
use layer_shika_domain::value_objects::popup_behavior::ConstraintAdjustment;
use layer_shika_domain::value_objects::popup_config::PopupConfig;
use layer_shika_domain::value_objects::popup_position::{
    Alignment, AnchorPoint, Offset, PopupPosition,
};
use layer_shika_domain::value_objects::popup_size::PopupSize;

/// Builder for configuring popups
///
/// Produces a [`PopupConfig`] and can show it via [`PopupShell`].
///
/// # Example
/// ```rust,ignore
/// shell.on("Main", "open_menu", |control| {
///     let popup_handle = control.popups().builder("MenuPopup")
///         .at_cursor()
///         .grab(true)
///         .close_on("menu_closed")
///         .show()?;
/// });
/// ```
pub struct PopupBuilder {
    shell: Option<PopupShell>,
    config: PopupConfig,
}

impl PopupBuilder {
    /// Creates a new popup builder for the specified component
    #[must_use]
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            shell: None,
            config: PopupConfig::new(component),
        }
    }

    #[must_use]
    pub(crate) fn with_shell(mut self, shell: PopupShell) -> Self {
        self.shell = Some(shell);
        self
    }

    #[must_use]
    pub fn position(mut self, position: PopupPosition) -> Self {
        self.config.position = position;
        self
    }

    #[must_use]
    pub fn at_cursor(self) -> Self {
        self.position(PopupPosition::Cursor {
            offset: Offset::default(),
        })
    }

    #[must_use]
    pub fn at_position(self, x: f32, y: f32) -> Self {
        self.position(PopupPosition::Absolute { x, y })
    }

    #[must_use]
    pub fn centered(self) -> Self {
        self.position(PopupPosition::Centered {
            offset: Offset::default(),
        })
    }

    #[must_use]
    pub fn relative_to_rect(
        self,
        rect: LogicalRect,
        anchor: AnchorPoint,
        alignment: Alignment,
    ) -> Self {
        self.position(PopupPosition::Element {
            rect,
            anchor,
            alignment,
        })
    }

    #[must_use]
    pub fn offset(mut self, x: f32, y: f32) -> Self {
        match &mut self.config.position {
            PopupPosition::Absolute { x: abs_x, y: abs_y } => {
                *abs_x += x;
                *abs_y += y;
            }
            PopupPosition::Cursor { offset }
            | PopupPosition::Centered { offset }
            | PopupPosition::RelativeToParent { offset, .. } => {
                offset.x += x;
                offset.y += y;
            }
            PopupPosition::Element { .. } => {
                self.config.position = PopupPosition::Cursor {
                    offset: Offset { x, y },
                };
            }
        }
        self
    }

    #[must_use]
    pub fn size(mut self, size: PopupSize) -> Self {
        self.config.size = size;
        self
    }

    #[must_use]
    pub fn fixed_size(self, width: f32, height: f32) -> Self {
        self.size(PopupSize::Fixed { width, height })
    }

    #[must_use]
    pub fn min_size(self, width: f32, height: f32) -> Self {
        self.size(PopupSize::Minimum { width, height })
    }

    #[must_use]
    pub fn max_size(self, width: f32, height: f32) -> Self {
        self.size(PopupSize::Maximum { width, height })
    }

    #[must_use]
    pub fn content_sized(self) -> Self {
        self.size(PopupSize::Content)
    }

    #[must_use]
    pub fn grab(mut self, enable: bool) -> Self {
        self.config.behavior.grab = enable;
        self
    }

    #[must_use]
    pub fn modal(mut self, enable: bool) -> Self {
        self.config.behavior.modal = enable;
        self
    }

    #[must_use]
    pub fn close_on_click_outside(mut self) -> Self {
        self.config.behavior.close_on_click_outside = true;
        self
    }

    #[must_use]
    pub fn close_on_escape(mut self) -> Self {
        self.config.behavior.close_on_escape = true;
        self
    }

    #[must_use]
    pub fn constraint_adjustment(mut self, adjustment: ConstraintAdjustment) -> Self {
        self.config.behavior.constraint_adjustment = adjustment;
        self
    }

    #[must_use]
    pub fn on_output(mut self, target: OutputTarget) -> Self {
        self.config.output = target;
        self
    }

    #[must_use]
    pub fn on_primary(self) -> Self {
        self.on_output(OutputTarget::Primary)
    }

    #[must_use]
    pub fn on_active(self) -> Self {
        self.on_output(OutputTarget::Active)
    }

    #[must_use]
    pub fn parent(mut self, parent: PopupHandle) -> Self {
        self.config.parent = Some(parent);
        self
    }

    #[must_use]
    pub const fn z_index(mut self, index: i32) -> Self {
        self.config.z_index = index;
        self
    }

    #[must_use]
    pub fn close_on(mut self, callback_name: impl Into<String>) -> Self {
        self.config.close_callback = Some(callback_name.into());
        self
    }

    #[must_use]
    pub fn resize_on(mut self, callback_name: impl Into<String>) -> Self {
        self.config.resize_callback = Some(callback_name.into());
        self
    }

    #[must_use]
    pub fn build(self) -> PopupConfig {
        self.config
    }

    pub fn show(self) -> Result<PopupHandle> {
        let Some(shell) = self.shell else {
            return Err(Error::Domain(DomainError::Configuration {
                message: "PopupBuilder::show() requires a builder created via `shell.popups().builder(...)`".to_string(),
            }));
        };
        shell.show(self.config)
    }

    pub fn show_with_shell(self, shell: &PopupShell) -> Result<PopupHandle> {
        shell.show(self.build())
    }
}
