use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_OUTPUT_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputHandle {
    id: usize,
}

impl OutputHandle {
    pub fn new() -> Self {
        Self {
            id: NEXT_OUTPUT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub const fn id(&self) -> usize {
        self.id
    }
}

impl Default for OutputHandle {
    fn default() -> Self {
        Self::new()
    }
}
