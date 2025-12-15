use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{PopupPlacement, PopupRequest, PopupSize};

/// Builder for configuring popup windows
///
/// This is a convenience wrapper around `PopupRequest::builder()` that provides
/// a fluent API for configuring popups. Once built, pass the resulting `PopupRequest`
/// to `ShellControl::show_popup()` from within a callback.
///
/// # Example
/// ```rust,ignore
/// shell.on("Main", "open_menu", |control| {
///     let request = PopupBuilder::new("MenuPopup")
///         .relative_to_cursor()
///         .anchor_top_left()
///         .grab(true)
///         .close_on("menu_closed")
///         .build();
///
///     control.show_popup(&request)?;
/// });
/// ```
pub struct PopupBuilder {
    component: String,
    reference: PopupPlacement,
    anchor: PopupPositioningMode,
    size: PopupSize,
    grab: bool,
    close_callback: Option<String>,
    resize_callback: Option<String>,
}

impl PopupBuilder {
    /// Creates a new popup builder for the specified component
    #[must_use]
    pub fn new(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            reference: PopupPlacement::AtCursor,
            anchor: PopupPositioningMode::TopLeft,
            size: PopupSize::Content,
            grab: false,
            close_callback: None,
            resize_callback: None,
        }
    }

    /// Positions the popup at the current cursor location
    #[must_use]
    pub fn relative_to_cursor(mut self) -> Self {
        self.reference = PopupPlacement::AtCursor;
        self
    }

    /// Positions the popup at the specified coordinates
    #[must_use]
    pub fn relative_to_point(mut self, x: f32, y: f32) -> Self {
        self.reference = PopupPlacement::AtPosition { x, y };
        self
    }

    /// Positions the popup relative to a rectangular area
    #[must_use]
    pub fn relative_to_rect(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.reference = PopupPlacement::AtRect { x, y, w, h };
        self
    }

    /// Sets the anchor point for positioning the popup
    #[must_use]
    pub const fn anchor(mut self, anchor: PopupPositioningMode) -> Self {
        self.anchor = anchor;
        self
    }

    /// Anchors popup to top-left corner
    #[must_use]
    pub fn anchor_top_left(mut self) -> Self {
        self.anchor = PopupPositioningMode::TopLeft;
        self
    }

    /// Anchors popup to top-center
    #[must_use]
    pub fn anchor_top_center(mut self) -> Self {
        self.anchor = PopupPositioningMode::TopCenter;
        self
    }

    /// Anchors popup to top-right corner
    #[must_use]
    pub fn anchor_top_right(mut self) -> Self {
        self.anchor = PopupPositioningMode::TopRight;
        self
    }

    /// Anchors popup to center-left
    #[must_use]
    pub fn anchor_center_left(mut self) -> Self {
        self.anchor = PopupPositioningMode::CenterLeft;
        self
    }

    /// Anchors popup to center
    #[must_use]
    pub fn anchor_center(mut self) -> Self {
        self.anchor = PopupPositioningMode::Center;
        self
    }

    /// Anchors popup to center-right
    #[must_use]
    pub fn anchor_center_right(mut self) -> Self {
        self.anchor = PopupPositioningMode::CenterRight;
        self
    }

    /// Anchors popup to bottom-left corner
    #[must_use]
    pub fn anchor_bottom_left(mut self) -> Self {
        self.anchor = PopupPositioningMode::BottomLeft;
        self
    }

    /// Anchors popup to bottom-center
    #[must_use]
    pub fn anchor_bottom_center(mut self) -> Self {
        self.anchor = PopupPositioningMode::BottomCenter;
        self
    }

    /// Anchors popup to bottom-right corner
    #[must_use]
    pub fn anchor_bottom_right(mut self) -> Self {
        self.anchor = PopupPositioningMode::BottomRight;
        self
    }

    /// Sets the popup size strategy
    ///
    /// Use `PopupSize::Content` for auto-sizing or `PopupSize::Fixed { w, h }` for explicit dimensions.
    #[must_use]
    pub const fn size(mut self, size: PopupSize) -> Self {
        self.size = size;
        self
    }

    /// Sets a fixed size for the popup
    #[must_use]
    pub fn fixed_size(mut self, w: f32, h: f32) -> Self {
        self.size = PopupSize::Fixed { w, h };
        self
    }

    /// Uses content-based sizing for the popup
    #[must_use]
    pub fn content_size(mut self) -> Self {
        self.size = PopupSize::Content;
        self
    }

    /// Enables or disables keyboard/pointer grab for modal behavior
    #[must_use]
    pub const fn grab(mut self, enable: bool) -> Self {
        self.grab = enable;
        self
    }

    /// Registers a callback that will close the popup when invoked
    #[must_use]
    pub fn close_on(mut self, callback_name: impl Into<String>) -> Self {
        self.close_callback = Some(callback_name.into());
        self
    }

    /// Registers a callback that will resize the popup when invoked
    #[must_use]
    pub fn resize_on(mut self, callback_name: impl Into<String>) -> Self {
        self.resize_callback = Some(callback_name.into());
        self
    }

    /// Builds the popup request
    ///
    /// After building, pass the request to `ShellControl::show_popup()` to display the popup.
    #[must_use]
    pub fn build(self) -> PopupRequest {
        let mut builder = PopupRequest::builder(self.component.clone())
            .placement(self.reference)
            .size(self.size)
            .mode(self.anchor)
            .grab(self.grab);

        if let Some(ref close_cb) = self.close_callback {
            builder = builder.close_on(close_cb.clone());
        }

        if let Some(ref resize_cb) = self.resize_callback {
            builder = builder.resize_on(resize_cb.clone());
        }

        builder.build()
    }
}
