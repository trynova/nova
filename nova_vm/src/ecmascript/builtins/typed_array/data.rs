use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone, Default)]
pub struct TypedArrayHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
