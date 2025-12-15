#[derive(Debug, Clone, Default)]
pub enum PopupSize {
    /// Fixed logical size
    Fixed { width: f32, height: f32 },

    /// Minimum size (can grow with content)
    Minimum { width: f32, height: f32 },

    /// Maximum size (can shrink below content)
    Maximum { width: f32, height: f32 },

    /// Constrained range
    Range {
        min_width: f32,
        min_height: f32,
        max_width: f32,
        max_height: f32,
    },

    /// Automatic based on content (default: use 2×2 initialization)
    #[default]
    Content,

    /// Match parent popup size
    MatchParent,
}
