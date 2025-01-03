// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [25.1 ArrayBuffer Objects](https://tc39.es/ecma262/#sec-arraybuffer-objects)

mod abstract_operations;
mod data;
use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::ArrayBufferIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

use abstract_operations::detach_array_buffer;
pub(crate) use abstract_operations::{
    allocate_array_buffer, array_buffer_byte_length, clone_array_buffer, get_value_from_buffer,
    is_detached_buffer, is_fixed_length_array_buffer, set_value_in_buffer, Ordering, DetachKey,
};
pub use data::*;
use std::ops::{Index, IndexMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ArrayBuffer(ArrayBufferIndex);

impl ArrayBuffer {
    #[inline]
    pub fn is_detached(self, agent: &Agent) -> bool {
        agent[self].is_detached()
    }

    #[inline]
    pub fn is_resizable(self, agent: &Agent) -> bool {
        agent[self].is_resizable()
    }

    #[inline]
    pub fn byte_length(self, agent: &Agent) -> usize {
        agent[self].byte_length()
    }

    #[inline]
    pub fn max_byte_length(self, agent: &Agent) -> usize {
        agent[self].max_byte_length()
    }

    pub fn detach(self, agent: &mut Agent, key: Option<DetachKey>) {
        detach_array_buffer(agent, self, key);
    }

    /// Resize a Resizable ArrayBuffer.
    ///
    /// `new_byte_length` must be a safe integer.
    pub(crate) fn resize(self, agent: &mut Agent, new_byte_length: usize) {
        agent[self].resize(new_byte_length);
    }

    /// Copy data from `source` ArrayBuffer to this ArrayBuffer.
    ///
    /// `self` and `source` must be different ArrayBuffers.
    pub(crate) fn copy_array_buffer_data(
        self,
        agent: &mut Agent,
        source: ArrayBuffer,
        first: usize,
        count: usize,
    ) {
        debug_assert_ne!(self, source);
        let array_buffers = &mut *agent.heap.array_buffers;
        let (source_data, target_data) = if self.get_index() > source.get_index() {
            let (before, after) = array_buffers.split_at_mut(self.get_index());
            (
                before[source.get_index()].as_ref().unwrap(),
                after[0].as_mut().unwrap(),
            )
        } else {
            let (before, after) = array_buffers.split_at_mut(source.get_index());
            (
                after[0].as_ref().unwrap(),
                before[self.get_index()].as_mut().unwrap(),
            )
        };
        let source_data = source_data.buffer.get_data_block();
        let target_data = target_data.buffer.get_data_block_mut();
        target_data.copy_data_block_bytes(0, source_data, first, count);
    }

    pub(crate) const fn _def() -> Self {
        Self(ArrayBufferIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoObject for ArrayBuffer {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoValue for ArrayBuffer {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl TryFrom<Value> for ArrayBuffer {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::ArrayBuffer(base_index) => Ok(base_index),
            _ => Err(()),
        }
    }
}

impl From<ArrayBufferIndex> for ArrayBuffer {
    fn from(value: ArrayBufferIndex) -> Self {
        ArrayBuffer(value)
    }
}

impl From<ArrayBuffer> for Object {
    fn from(value: ArrayBuffer) -> Self {
        Self::ArrayBuffer(value)
    }
}

impl From<ArrayBuffer> for Value {
    fn from(value: ArrayBuffer) -> Self {
        Self::ArrayBuffer(value)
    }
}

impl Index<ArrayBuffer> for Agent {
    type Output = ArrayBufferHeapData;

    fn index(&self, index: ArrayBuffer) -> &Self::Output {
        &self.heap.array_buffers[index]
    }
}

impl IndexMut<ArrayBuffer> for Agent {
    fn index_mut(&mut self, index: ArrayBuffer) -> &mut Self::Output {
        &mut self.heap.array_buffers[index]
    }
}

impl Index<ArrayBuffer> for Vec<Option<ArrayBufferHeapData>> {
    type Output = ArrayBufferHeapData;

    fn index(&self, index: ArrayBuffer) -> &Self::Output {
        self.get(index.get_index())
            .expect("ArrayBuffer out of bounds")
            .as_ref()
            .expect("ArrayBuffer slot empty")
    }
}

impl IndexMut<ArrayBuffer> for Vec<Option<ArrayBufferHeapData>> {
    fn index_mut(&mut self, index: ArrayBuffer) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ArrayBuffer out of bounds")
            .as_mut()
            .expect("ArrayBuffer slot empty")
    }
}

impl InternalSlots for ArrayBuffer {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::ArrayBuffer;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }
}

impl InternalMethods for ArrayBuffer {}

impl HeapMarkAndSweep for ArrayBuffer {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.array_buffers.shift_index(&mut self.0);
    }
}

impl CreateHeapData<ArrayBufferHeapData, ArrayBuffer> for Heap {
    fn create(&mut self, data: ArrayBufferHeapData) -> ArrayBuffer {
        self.array_buffers.push(Some(data));
        ArrayBuffer::from(ArrayBufferIndex::last(&self.array_buffers))
    }
}
