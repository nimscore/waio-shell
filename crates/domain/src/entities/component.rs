#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UiComponentHandle(usize);

impl UiComponentHandle {
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn id(&self) -> usize {
        self.0
    }
}
