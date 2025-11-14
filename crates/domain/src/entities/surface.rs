#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceHandle(usize);

impl SurfaceHandle {
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn id(&self) -> usize {
        self.0
    }
}
