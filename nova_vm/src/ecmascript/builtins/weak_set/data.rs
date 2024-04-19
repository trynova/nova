use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct WeakSetHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
