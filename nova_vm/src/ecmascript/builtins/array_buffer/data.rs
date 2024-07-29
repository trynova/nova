// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
pub struct ArrayBufferHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    pub(super) buffer: InternalBuffer,
    // detach_key
}

impl Default for ArrayBufferHeapData<'_> {
    #[inline(always)]
    fn default() -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::Detached,
        }
    }
}

unsafe impl Send for ArrayBufferHeapData<'_> {}

impl ArrayBufferHeapData<'_> {
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

impl<'gen> HeapMarkAndSweep<'gen> for ArrayBufferHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
