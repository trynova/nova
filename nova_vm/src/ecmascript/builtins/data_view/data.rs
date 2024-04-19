use crate::heap::indexes::ObjectIndex;

#[derive(Debug, Clone)]
pub struct DataViewHeapData {
    pub(crate) object_index: Option<ObjectIndex>,
}
