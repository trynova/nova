use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone, Default)]
pub struct DataViewHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
