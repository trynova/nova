use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct ModuleHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
