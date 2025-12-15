/// Wayland-compatible constraint adjustment
///
/// Maps directly to `XdgPositioner` `constraint_adjustment` flags. Compositor
/// handles the actual repositioning logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConstraintAdjustment {
    /// No adjustment (manual clamping)
    #[default]
    None,

    /// Slide along axis to fit
    Slide,

    /// Flip to opposite side if doesn't fit
    Flip,

    /// Resize to fit
    Resize,

    /// Combination strategies
    SlideAndResize,
    FlipAndSlide,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMigrationPolicy {
    /// Move to primary when output disconnects
    #[default]
    MigrateToPrimary,

    /// Move to currently active output
    MigrateToActive,

    /// Close when output disconnects
    Close,
}

/// Behavioral configuration
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct PopupBehavior {
    /// Grab keyboard and pointer input
    pub grab: bool,

    /// Modal (blocks interaction with parent)
    pub modal: bool,

    /// Auto-close on outside click
    pub close_on_click_outside: bool,

    /// Auto-close on escape key
    pub close_on_escape: bool,

    /// How to handle screen edge constraints
    pub constraint_adjustment: ConstraintAdjustment,

    /// How to handle output disconnect for this popup
    pub output_migration: OutputMigrationPolicy,
}
