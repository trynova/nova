use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct TypedArrayHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
