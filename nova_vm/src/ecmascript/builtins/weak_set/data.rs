use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone, Default)]
pub struct WeakSetHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
