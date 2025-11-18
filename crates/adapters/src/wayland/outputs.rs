use layer_shika_domain::value_objects::output_handle::OutputHandle;
use std::collections::HashMap;
use wayland_client::backend::ObjectId;

pub struct OutputMapping {
    object_to_handle: HashMap<ObjectId, OutputHandle>,
}

impl OutputMapping {
    pub fn new() -> Self {
        Self {
            object_to_handle: HashMap::new(),
        }
    }

    pub fn insert(&mut self, object_id: ObjectId) -> OutputHandle {
        let handle = OutputHandle::new();
        self.object_to_handle.insert(object_id, handle);
        handle
    }

    pub fn get(&self, object_id: &ObjectId) -> Option<OutputHandle> {
        self.object_to_handle.get(object_id).copied()
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, object_id: &ObjectId) -> Option<OutputHandle> {
        self.object_to_handle.remove(object_id)
    }
}

impl Default for OutputMapping {
    fn default() -> Self {
        Self::new()
    }
}
