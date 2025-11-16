use wayland_client::backend::ObjectId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputKey(ObjectId);

impl OutputKey {
    pub const fn new(id: ObjectId) -> Self {
        Self(id)
    }

    pub const fn id(&self) -> &ObjectId {
        &self.0
    }
}

impl From<ObjectId> for OutputKey {
    fn from(id: ObjectId) -> Self {
        Self::new(id)
    }
}
