use crate::ecmascript::types::OrdinaryObject;

#[derive(Debug, Clone, Default)]
pub struct WeakSetHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}
