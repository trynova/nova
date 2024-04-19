use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct SharedArrayBufferHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
