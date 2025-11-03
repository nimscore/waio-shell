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
    pub at: PopupAt,
    pub size: PopupSize,
    pub mode: PopupPositioningMode,
}

impl PopupRequest {
    #[must_use]
    pub fn new(
        component: String,
        at: PopupAt,
        size: PopupSize,
        mode: PopupPositioningMode,
    ) -> Self {
        Self {
            component,
            at,
            size,
            mode,
        }
    }

    #[must_use]
    pub fn builder(component: String) -> PopupRequestBuilder {
        PopupRequestBuilder::new(component)
    }
}

#[derive(Debug, Clone)]
pub enum PopupAt {
    Absolute { x: f32, y: f32 },
    Cursor,
    AnchorRect { x: f32, y: f32, w: f32, h: f32 },
}

impl PopupAt {
    #[must_use]
    pub const fn absolute(x: f32, y: f32) -> Self {
        Self::Absolute { x, y }
    }

    #[must_use]
    pub const fn cursor() -> Self {
        Self::Cursor
    }

    #[must_use]
    pub const fn anchor_rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::AnchorRect { x, y, w, h }
    }

    #[must_use]
    pub const fn position(&self) -> (f32, f32) {
        match *self {
            Self::Absolute { x, y } | Self::AnchorRect { x, y, .. } => (x, y),
            Self::Cursor => (0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone)]
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
    at: PopupAt,
    size: PopupSize,
    mode: PopupPositioningMode,
}

impl PopupRequestBuilder {
    #[must_use]
    pub fn new(component: String) -> Self {
        Self {
            component,
            at: PopupAt::Cursor,
            size: PopupSize::Content,
            mode: PopupPositioningMode::default(),
        }
    }

    #[must_use]
    pub const fn at(mut self, at: PopupAt) -> Self {
        self.at = at;
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
    pub fn build(self) -> PopupRequest {
        PopupRequest {
            component: self.component,
            at: self.at,
            size: self.size,
            mode: self.mode,
        }
    }
}
