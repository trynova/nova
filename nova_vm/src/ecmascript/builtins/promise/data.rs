use crate::{ecmascript::types::OrdinaryObject};

#[derive(Debug, Clone, Default)]
pub struct PromiseHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}
