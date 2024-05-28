use crate::{ecmascript::types::OrdinaryObject};

#[derive(Debug, Clone, Default)]
pub struct SharedArrayBufferHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}
