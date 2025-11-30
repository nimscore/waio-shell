use super::popup_positioning_mode::PopupPositioningMode;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PopupHandle(usize);

impl PopupHandle {
    #[must_use]
    pub const fn new(key: usize) -> Self {
        Self(key)
    }

    #[must_use]
    pub const fn key(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct PopupRequest {
    pub component: String,
    pub placement: PopupPlacement,
    pub size: PopupSize,
    pub mode: PopupPositioningMode,
    pub grab: bool,
    pub close_callback: Option<String>,
    pub resize_callback: Option<String>,
}

impl PopupRequest {
    #[must_use]
    pub fn new(
        component: String,
        placement: PopupPlacement,
        size: PopupSize,
        mode: PopupPositioningMode,
    ) -> Self {
        Self {
            component,
            placement,
            size,
            mode,
            grab: false,
            close_callback: None,
            resize_callback: None,
        }
    }

    #[must_use]
    pub fn builder(component: String) -> PopupRequestBuilder {
        PopupRequestBuilder::new(component)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PopupPlacement {
    AtPosition { x: f32, y: f32 },
    AtCursor,
    AtRect { x: f32, y: f32, w: f32, h: f32 },
}

impl PopupPlacement {
    #[must_use]
    pub const fn at_position(x: f32, y: f32) -> Self {
        Self::AtPosition { x, y }
    }

    #[must_use]
    pub const fn at_cursor() -> Self {
        Self::AtCursor
    }

    #[must_use]
    pub const fn at_rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::AtRect { x, y, w, h }
    }

    #[must_use]
    pub const fn position(&self) -> (f32, f32) {
        match *self {
            Self::AtPosition { x, y } | Self::AtRect { x, y, .. } => (x, y),
            Self::AtCursor => (0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PopupSize {
    Fixed { w: f32, h: f32 },
    Content,
}

impl PopupSize {
    #[must_use]
    pub const fn fixed(w: f32, h: f32) -> Self {
        Self::Fixed { w, h }
    }

    #[must_use]
    pub const fn content() -> Self {
        Self::Content
    }

    #[must_use]
    pub const fn dimensions(&self) -> Option<(f32, f32)> {
        match *self {
            Self::Fixed { w, h } => Some((w, h)),
            Self::Content => None,
        }
    }
}

pub struct PopupRequestBuilder {
    component: String,
    placement: PopupPlacement,
    size: PopupSize,
    mode: PopupPositioningMode,
    grab: bool,
    close_callback: Option<String>,
    resize_callback: Option<String>,
}

impl PopupRequestBuilder {
    #[must_use]
    pub fn new(component: String) -> Self {
        Self {
            component,
            placement: PopupPlacement::AtCursor,
            size: PopupSize::Content,
            mode: PopupPositioningMode::default(),
            grab: false,
            close_callback: None,
            resize_callback: None,
        }
    }

    #[must_use]
    pub const fn placement(mut self, placement: PopupPlacement) -> Self {
        self.placement = placement;
        self
    }

    #[must_use]
    pub const fn size(mut self, size: PopupSize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn mode(mut self, mode: PopupPositioningMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub const fn grab(mut self, grab: bool) -> Self {
        self.grab = grab;
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

    #[must_use]
    pub fn build(self) -> PopupRequest {
        PopupRequest {
            component: self.component,
            placement: self.placement,
            size: self.size,
            mode: self.mode,
            grab: self.grab,
            close_callback: self.close_callback,
            resize_callback: self.resize_callback,
        }
    }
}
