use layer_shika_domain::value_objects::output_handle::OutputHandle;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use wayland_client::backend::ObjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputKey {
    handle: OutputHandle,
    _object_id_hash: u64,
}

impl OutputKey {
    pub fn new(object_id: &ObjectId) -> Self {
        let mut hasher = DefaultHasher::new();
        object_id.hash(&mut hasher);
        let object_id_hash = hasher.finish();

        Self {
            handle: OutputHandle::new(),
            _object_id_hash: object_id_hash,
        }
    }

    pub const fn handle(&self) -> OutputHandle {
        self.handle
    }
}

impl From<ObjectId> for OutputKey {
    fn from(id: ObjectId) -> Self {
        Self::new(&id)
    }
}

impl From<OutputHandle> for OutputKey {
    fn from(handle: OutputHandle) -> Self {
        Self {
            handle,
            _object_id_hash: 0,
        }
    }
}
