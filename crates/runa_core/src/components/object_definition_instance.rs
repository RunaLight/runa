use runa_macros::Scriptable;

#[derive(Clone, Debug, Scriptable)]
pub struct ObjectDefinitionInstance {
    pub object_id: String,
}

impl ObjectDefinitionInstance {
    pub fn new(object_id: impl Into<String>) -> Self {
        Self {
            object_id: object_id.into(),
        }
    }
}
