#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardInteractivity {
    None,
    Exclusive,
    OnDemand,
}

impl Default for KeyboardInteractivity {
    fn default() -> Self {
        Self::OnDemand
    }
}
