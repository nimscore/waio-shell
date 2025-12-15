use crate::value_objects::output_handle::OutputHandle;

/// Explicit output targeting
#[derive(Debug, Clone)]
pub enum OutputTarget {
    /// Use primary output
    Primary,

    /// Use currently active output
    Active,

    /// Use specific output by handle
    Handle(OutputHandle),

    /// Use output by name
    Named(String),

    /// Inherit from parent (for child popups)
    InheritFromParent,

    /// Use output containing cursor
    ContainingCursor,
}
