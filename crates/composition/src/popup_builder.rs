use crate::Result;
use crate::system::SingleWindowShell;
use layer_shika_adapters::platform::slint_interpreter::Value;
use layer_shika_domain::prelude::AnchorStrategy;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{PopupPlacement, PopupRequest, PopupSize};

pub struct PopupBuilder<'a> {
    shell: &'a SingleWindowShell,
    component: String,
    reference: PopupPlacement,
    anchor: PopupPositioningMode,
    size: PopupSize,
    grab: bool,
    close_callback: Option<String>,
    resize_callback: Option<String>,
}

impl<'a> PopupBuilder<'a> {
    pub(crate) fn new(shell: &'a SingleWindowShell, component: String) -> Self {
        Self {
            shell,
            component,
            reference: PopupPlacement::AtCursor,
            anchor: PopupPositioningMode::TopLeft,
            size: PopupSize::Content,
            grab: false,
            close_callback: None,
            resize_callback: None,
        }
    }

    #[must_use]
    pub fn relative_to_cursor(mut self) -> Self {
        self.reference = PopupPlacement::AtCursor;
        self
    }

    #[must_use]
    pub fn relative_to_point(mut self, x: f32, y: f32) -> Self {
        self.reference = PopupPlacement::AtPosition { x, y };
        self
    }

    #[must_use]
    pub fn relative_to_rect(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.reference = PopupPlacement::AtRect { x, y, w, h };
        self
    }

    #[must_use]
    pub const fn anchor(mut self, anchor: PopupPositioningMode) -> Self {
        self.anchor = anchor;
        self
    }

    #[must_use]
    pub fn anchor_top_left(mut self) -> Self {
        self.anchor = PopupPositioningMode::TopLeft;
        self
    }

    #[must_use]
    pub fn anchor_top_center(mut self) -> Self {
        self.anchor = PopupPositioningMode::TopCenter;
        self
    }

    #[must_use]
    pub fn anchor_top_right(mut self) -> Self {
        self.anchor = PopupPositioningMode::TopRight;
        self
    }

    #[must_use]
    pub fn anchor_center_left(mut self) -> Self {
        self.anchor = PopupPositioningMode::CenterLeft;
        self
    }

    #[must_use]
    pub fn anchor_center(mut self) -> Self {
        self.anchor = PopupPositioningMode::Center;
        self
    }

    #[must_use]
    pub fn anchor_center_right(mut self) -> Self {
        self.anchor = PopupPositioningMode::CenterRight;
        self
    }

    #[must_use]
    pub fn anchor_bottom_left(mut self) -> Self {
        self.anchor = PopupPositioningMode::BottomLeft;
        self
    }

    #[must_use]
    pub fn anchor_bottom_center(mut self) -> Self {
        self.anchor = PopupPositioningMode::BottomCenter;
        self
    }

    #[must_use]
    pub fn anchor_bottom_right(mut self) -> Self {
        self.anchor = PopupPositioningMode::BottomRight;
        self
    }

    #[must_use]
    pub const fn size(mut self, size: PopupSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn fixed_size(mut self, w: f32, h: f32) -> Self {
        self.size = PopupSize::Fixed { w, h };
        self
    }

    #[must_use]
    pub fn content_size(mut self) -> Self {
        self.size = PopupSize::Content;
        self
    }

    #[must_use]
    pub const fn grab(mut self, enable: bool) -> Self {
        self.grab = enable;
        self
    }

    #[must_use]
    pub fn close_on(mut self, callback_name: impl Into<String>) -> Self {
        self.close_callback = Some(callback_name.into());
        self
    }

    #[must_use]
    pub fn resize_on(mut self, callback_name: impl Into<String>) -> Self {
        self.resize_callback = Some(callback_name.into());
        self
    }

    pub fn bind(self, trigger_callback: &str) -> Result<()> {
        let request = self.build_request();
        let control = self.shell.control();

        self.shell.with_all_component_instances(|instance| {
            let request_clone = request.clone();
            let control_clone = control.clone();

            if let Err(e) = instance.set_callback(trigger_callback, move |_args| {
                if let Err(e) = control_clone.show_popup(&request_clone) {
                    log::error!("Failed to show popup: {}", e);
                }
                Value::Void
            }) {
                log::error!(
                    "Failed to bind popup callback '{}': {}",
                    trigger_callback,
                    e
                );
            }
        });

        Ok(())
    }

    pub fn toggle(self, trigger_callback: &str) -> Result<()> {
        let request = self.build_request();
        let control = self.shell.control();
        let component_name = request.component.clone();

        self.shell.with_all_component_instances(|instance| {
            let request_clone = request.clone();
            let control_clone = control.clone();
            let component_clone = component_name.clone();

            if let Err(e) = instance.set_callback(trigger_callback, move |_args| {
                log::debug!("Toggle callback for component: {}", component_clone);
                if let Err(e) = control_clone.show_popup(&request_clone) {
                    log::error!("Failed to toggle popup: {}", e);
                }
                Value::Void
            }) {
                log::error!(
                    "Failed to bind toggle popup callback '{}': {}",
                    trigger_callback,
                    e
                );
            }
        });

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub fn bind_anchored(self, trigger_callback: &str, strategy: AnchorStrategy) -> Result<()> {
        let component_name = self.component.clone();
        let grab = self.grab;
        let close_callback = self.close_callback.clone();
        let resize_callback = self.resize_callback.clone();
        let control = self.shell.control();

        self.shell.with_all_component_instances(|instance| {
            let component_clone = component_name.clone();
            let control_clone = control.clone();
            let close_cb = close_callback.clone();
            let resize_cb = resize_callback.clone();

            if let Err(e) = instance.set_callback(trigger_callback, move |args| {
                if args.len() < 4 {
                    log::error!(
                        "bind_anchored callback expects 4 arguments (x, y, width, height), got {}",
                        args.len()
                    );
                    return Value::Void;
                }

                let anchor_x = args
                    .first()
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);
                let anchor_y = args
                    .get(1)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);
                let anchor_w = args
                    .get(2)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);
                let anchor_h = args
                    .get(3)
                    .and_then(|v| v.clone().try_into().ok())
                    .unwrap_or(0.0);

                log::debug!(
                    "Anchored popup triggered for '{}' at rect: ({}, {}, {}, {})",
                    component_clone,
                    anchor_x,
                    anchor_y,
                    anchor_w,
                    anchor_h
                );

                let (reference_x, reference_y, mode) = match strategy {
                    AnchorStrategy::CenterBottom => {
                        let center_x = anchor_x + anchor_w / 2.0;
                        let bottom_y = anchor_y + anchor_h;
                        (center_x, bottom_y, PopupPositioningMode::TopCenter)
                    }
                    AnchorStrategy::CenterTop => {
                        let center_x = anchor_x + anchor_w / 2.0;
                        (center_x, anchor_y, PopupPositioningMode::BottomCenter)
                    }
                    AnchorStrategy::RightBottom => {
                        let right_x = anchor_x + anchor_w;
                        let bottom_y = anchor_y + anchor_h;
                        (right_x, bottom_y, PopupPositioningMode::TopRight)
                    }
                    AnchorStrategy::LeftTop => {
                        (anchor_x, anchor_y, PopupPositioningMode::BottomLeft)
                    }
                    AnchorStrategy::RightTop => {
                        let right_x = anchor_x + anchor_w;
                        (right_x, anchor_y, PopupPositioningMode::BottomRight)
                    }
                    AnchorStrategy::LeftBottom => {
                        let bottom_y = anchor_y + anchor_h;
                        (anchor_x, bottom_y, PopupPositioningMode::TopLeft)
                    }
                    AnchorStrategy::Cursor => (anchor_x, anchor_y, PopupPositioningMode::TopLeft),
                };

                log::debug!(
                    "Resolved anchored popup reference for '{}' -> ({}, {}), mode: {:?}",
                    component_clone,
                    reference_x,
                    reference_y,
                    mode
                );

                let mut builder = PopupRequest::builder(component_clone.clone())
                    .placement(PopupPlacement::at_position(reference_x, reference_y))
                    .size(PopupSize::Content)
                    .grab(grab)
                    .mode(mode);

                if let Some(ref close_cb) = close_cb {
                    builder = builder.close_on(close_cb.clone());
                }

                if let Some(ref resize_cb) = resize_cb {
                    builder = builder.resize_on(resize_cb.clone());
                }

                let request = builder.build();

                if let Err(e) = control_clone.show_popup(&request) {
                    log::error!("Failed to show anchored popup: {}", e);
                }

                Value::Void
            }) {
                log::error!(
                    "Failed to bind anchored popup callback '{}': {}",
                    trigger_callback,
                    e
                );
            }
        });

        Ok(())
    }

    fn build_request(&self) -> PopupRequest {
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
