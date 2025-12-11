/// Controls how a surface receives keyboard input
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardInteractivity {
    /// Surface does not receive keyboard events
    None,
    /// Surface always receives keyboard focus
    Exclusive,
    /// Surface receives focus when clicked (default)
    OnDemand,
}

impl Default for KeyboardInteractivity {
    fn default() -> Self {
        Self::OnDemand
    }
}
