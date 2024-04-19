use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct PromiseHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
