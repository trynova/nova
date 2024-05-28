use crate::ecmascript::types::OrdinaryObject;

#[derive(Debug, Clone, Default)]
pub struct TypedArrayHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}
