use crate::{ecmascript::types::OrdinaryObject};

#[derive(Debug, Clone, Default)]
pub struct FinalizationRegistryHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}
