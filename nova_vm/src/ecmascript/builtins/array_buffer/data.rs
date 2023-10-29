use crate::{ecmascript::types::DataBlock, heap::indexes::ObjectIndex};

#[derive(Debug)]
pub(crate) enum InternalBuffer {
    Detached,
    Growable(DataBlock),
    Static(DataBlock),
    // Shared(SharedDataBlockIndex)
}

#[derive(Debug)]
pub struct ArrayBufferHeapData {
    pub(super) object_index: Option<ObjectIndex>,
    pub(super) buffer: InternalBuffer,
    // detach_key
}
