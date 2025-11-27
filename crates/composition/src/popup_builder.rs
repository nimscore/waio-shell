use crate::Result;
use crate::system::App;
use layer_shika_adapters::platform::slint_interpreter::Value;
use layer_shika_domain::prelude::AnchorStrategy;
use layer_shika_domain::value_objects::popup_positioning_mode::PopupPositioningMode;
use layer_shika_domain::value_objects::popup_request::{PopupAt, PopupRequest, PopupSize};

pub struct PopupBuilder<'a> {
    app: &'a App,
    component: String,
    reference: PopupAt,
    anchor: PopupPositioningMode,
    size: PopupSize,
    grab: bool,
    close_callback: Option<String>,
    resize_callback: Option<String>,
}

impl<'a> PopupBuilder<'a> {
    pub(crate) fn new(app: &'a App, component: String) -> Self {
        Self {
            app,
            component,
            reference: PopupAt::Cursor,
            anchor: PopupPositioningMode::TopLeft,
            size: PopupSize::Content,
            grab: false,
            close_callback: None,
            resize_callback: None,
        }
    }

    #[must_use]
    pub fn relative_to_cursor(mut self) -> Self {
        self.reference = PopupAt::Cursor;
        self
    }

    #[must_use]
    pub fn relative_to_point(mut self, x: f32, y: f32) -> Self {
        self.reference = PopupAt::Absolute { x, y };
        self
    }

    #[must_use]
    pub fn relative_to_rect(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.reference = PopupAt::AnchorRect { x, y, w, h };
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
        let control = self.app.control();

        self.app.with_all_component_instances(|instance| {
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
        let control = self.app.control();
        let component_name = request.component.clone();

        self.app.with_all_component_instances(|instance| {
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

    pub fn bind_anchored(self, trigger_callback: &str, strategy: AnchorStrategy) -> Result<()> {
        let component_name = self.component.clone();
        let grab = self.grab;
        let close_callback = self.close_callback.clone();
        let resize_callback = self.resize_callback.clone();
        let control = self.app.control();

        self.app.with_all_component_instances(|instance| {
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

                let mut builder = PopupRequest::builder(component_clone.clone())
                    .at(PopupAt::AnchorRect {
                        x: anchor_x,
                        y: anchor_y,
                        w: anchor_w,
                        h: anchor_h,
                    })
                    .size(PopupSize::Content)
                    .grab(grab);

                let mode = match strategy {
                    AnchorStrategy::CenterBottom => PopupPositioningMode::TopCenter,
                    AnchorStrategy::CenterTop => PopupPositioningMode::BottomCenter,
                    AnchorStrategy::RightBottom => PopupPositioningMode::TopRight,
                    AnchorStrategy::LeftTop => PopupPositioningMode::BottomLeft,
                    AnchorStrategy::RightTop => PopupPositioningMode::BottomRight,
                    AnchorStrategy::LeftBottom | AnchorStrategy::Cursor => {
                        PopupPositioningMode::TopLeft
                    }
                };

                builder = builder.mode(mode);

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
            .at(self.reference)
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
