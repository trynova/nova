use std::mem::ManuallyDrop;

use crate::{
    ecmascript::types::{DataBlock, OrdinaryObject},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug)]
pub(crate) enum InternalBuffer {
    Detached,
    FixedLength(ManuallyDrop<DataBlock>),
    Resizable(ManuallyDrop<DataBlock>),
    // TODO: Implement SharedDataBlock
    SharedFixedLength(()),
    SharedResizableLength(()),
}

#[derive(Debug)]
pub struct ArrayBufferHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(super) buffer: InternalBuffer,
    // detach_key
}

impl Default for ArrayBufferHeapData {
    #[inline(always)]
    fn default() -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::Detached,
        }
    }
}

unsafe impl Send for ArrayBufferHeapData {}

impl ArrayBufferHeapData {
    pub(crate) fn new_resizable(db: DataBlock) -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::Resizable(ManuallyDrop::new(db)),
        }
    }

    pub(crate) fn new_fixed_length(db: DataBlock) -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::FixedLength(ManuallyDrop::new(db)),
        }
    }

    pub(crate) fn is_detached_buffer(&self) -> bool {
        matches!(self.buffer, InternalBuffer::Detached)
    }
}

impl HeapMarkAndSweep for ArrayBufferHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
