use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone, Default)]
pub struct SharedArrayBufferHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
