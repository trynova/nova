use crate::ecmascript::types::OrdinaryObject;

#[derive(Debug, Clone, Default)]
pub struct DataViewHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}
