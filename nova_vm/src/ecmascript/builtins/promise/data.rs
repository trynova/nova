use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone, Default)]
pub struct PromiseHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
