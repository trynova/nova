use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct FinalizationRegistryHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
