/// Width and height of a layer surface in pixels
///
/// According to the Wayland wlr-layer-shell protocol:
/// - Pass 0 for either width or height to have the compositor assign it
/// - The compositor will center the surface with respect to its anchors
/// - When using 0, you MUST anchor to opposite edges in that dimension:
///   - width = 0 requires both left and right anchors
///   - height = 0 requires both top and bottom anchors
/// - Not following this requirement is a protocol error
/// - Both values default to 0
///
/// Size is double-buffered via `wl_surface.commit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SurfaceDimension {
    width: u32,
    height: u32,
}

impl SurfaceDimension {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn from_raw(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupDimensions {
    pub width: f32,
    pub height: f32,
}

impl Default for PopupDimensions {
    fn default() -> Self {
        Self {
            width: 200.0,
            height: 150.0,
        }
    }
}

impl PopupDimensions {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(&self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> f32 {
        self.height
    }
}
