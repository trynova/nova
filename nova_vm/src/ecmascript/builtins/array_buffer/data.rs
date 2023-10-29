use crate::{ecmascript::types::DataBlock, heap::indexes::ObjectIndex};

#[derive(Debug)]
pub(crate) enum InternalBuffer {
    Detached,
    FixedLength(DataBlock),
    Resizable(DataBlock),
    // TODO: Implement SharedDataBlock
    SharedFixedLength(()),
    SharedResizableLength(()),
}

#[derive(Debug)]
pub struct ArrayBufferHeapData {
    pub(super) object_index: Option<ObjectIndex>,
    pub(super) buffer: InternalBuffer,
    // detach_key
}

impl ArrayBufferHeapData {
    pub(crate) fn new_resizable(db: DataBlock) -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::Resizable(db),
        }
    }

    pub(crate) fn new_fixed_length(db: DataBlock) -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::FixedLength(db),
        }
    }

    pub(crate) fn is_detached_buffer(&self) -> bool {
        matches!(self.buffer, InternalBuffer::Detached)
    }
}
