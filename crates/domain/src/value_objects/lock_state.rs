#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Inactive,
    Locking,
    Locked,
    Unlocking,
}

impl LockState {
    #[must_use]
    pub const fn can_activate(self) -> bool {
        matches!(self, Self::Inactive)
    }

    #[must_use]
    pub const fn can_deactivate(self) -> bool {
        matches!(self, Self::Locked | Self::Locking)
    }

    #[must_use]
    pub const fn is_transitioning(self) -> bool {
        matches!(self, Self::Locking | Self::Unlocking)
    }
}
