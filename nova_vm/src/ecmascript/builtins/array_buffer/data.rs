// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{DataBlock, OrdinaryObject},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug)]
pub(crate) struct InternalBuffer {
    data_block: DataBlock,
    /// Capacity of a resizable ArrayBuffer, or usize::MAX if the buffer is not
    /// resizable.
    capacity: usize,
}

impl InternalBuffer {
    /// Returns the contained DataBlock.
    ///
    /// Panics if the buffer is detached.
    pub(crate) fn get_data_block(&self) -> &DataBlock {
        if self.data_block.is_detached() {
            unreachable!();
        }
        &self.data_block
    }

    /// Returns the contained DataBlock.
    ///
    /// Panics if the buffer is detached.
    pub(crate) fn get_data_block_mut(&mut self) -> &mut DataBlock {
        if self.data_block.is_detached() {
            unreachable!();
        }
        &mut self.data_block
    }

    pub(crate) fn detach(&mut self) {
        self.capacity = 0;
        self.data_block = DataBlock::DETACHED_DATA_BLOCK;
    }

    const fn detached() -> Self {
        Self {
            data_block: DataBlock::DETACHED_DATA_BLOCK,
            capacity: usize::MAX,
        }
    }

    fn fixed_length(data_block: DataBlock) -> Self {
        Self {
            data_block,
            capacity: usize::MAX,
        }
    }

    fn resizable(data_block: DataBlock, capacity: usize) -> Self {
        assert_ne!(capacity, usize::MAX);
        Self {
            data_block,
            capacity,
        }
    }
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
            buffer: InternalBuffer::detached(),
        }
    }
}

unsafe impl Send for ArrayBufferHeapData {}

impl ArrayBufferHeapData {
    pub(crate) fn new_resizable(db: DataBlock, cap: usize) -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::resizable(db, cap),
        }
    }

    pub(crate) fn new_fixed_length(db: DataBlock) -> Self {
        Self {
            object_index: None,
            buffer: InternalBuffer::fixed_length(db),
        }
    }

    /// Returns the contained DataBlock.
    ///
    /// Panics if the buffer is detached.
    pub(crate) fn get_data_block(&self) -> &DataBlock {
        &self.buffer.data_block
    }

    /// Returns the contained DataBlock.
    ///
    /// Panics if the buffer is detached.
    pub(crate) fn get_data_block_mut(&mut self) -> &mut DataBlock {
        &mut self.buffer.data_block
    }

    pub(crate) fn is_detached(&self) -> bool {
        self.buffer.data_block.is_detached()
    }

    pub(crate) fn is_resizable(&self) -> bool {
        self.buffer.capacity != usize::MAX
    }

    pub(crate) fn byte_length(&self) -> usize {
        if self.is_detached() {
            0
        } else {
            self.buffer.data_block.len()
        }
    }

    pub(crate) fn max_byte_length(&self) -> usize {
        if self.is_detached() {
            0
        } else if self.is_resizable() {
            self.buffer.capacity
        } else {
            self.buffer.data_block.len()
        }
    }

    pub(crate) fn resize(&mut self, new_byte_length: usize) {
        if self.is_resizable() {
            self.buffer.data_block.realloc(new_byte_length);
        } else {
            unreachable!();
        }
    }
}

impl HeapMarkAndSweep for ArrayBufferHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            buffer: _,
        } = self;
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            buffer: _,
        } = self;
        object_index.sweep_values(compactions);
    }
}
