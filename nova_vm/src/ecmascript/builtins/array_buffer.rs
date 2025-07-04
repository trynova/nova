// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [25.1 ArrayBuffer Objects](https://tc39.es/ecma262/#sec-arraybuffer-objects)

mod abstract_operations;
mod data;
use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::ArrayBufferIndex,
    },
};

use abstract_operations::detach_array_buffer;
pub(crate) use abstract_operations::{
    DetachKey, Ordering, allocate_array_buffer, array_buffer_byte_length, clone_array_buffer,
    get_value_from_buffer, is_detached_buffer, is_fixed_length_array_buffer, set_value_in_buffer,
};
use core::ops::{Index, IndexMut};
pub use data::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ArrayBuffer<'a>(ArrayBufferIndex<'a>);

impl ArrayBuffer<'_> {
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

    #[inline]
    pub fn get_detach_key(self, agent: &Agent) -> Option<DetachKey> {
        agent.heap.array_buffer_detach_keys.get(&self).copied()
    }

    #[inline]
    pub fn set_detach_key(self, agent: &mut Agent, key: Option<DetachKey>) {
        if let Some(key) = key {
            agent.heap.alloc_counter += core::mem::size_of::<(ArrayBuffer, DetachKey)>();
            agent
                .heap
                .array_buffer_detach_keys
                .insert(self.unbind(), key);
        } else {
            agent.heap.array_buffer_detach_keys.remove(&self.unbind());
        }
    }

    pub fn detach<'a>(
        self,
        agent: &mut Agent,
        key: Option<DetachKey>,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        detach_array_buffer(agent, self, key, gc)
    }

    /// Resize a Resizable ArrayBuffer.
    ///
    /// `new_byte_length` must be a safe integer.
    pub(crate) fn resize(self, agent: &mut Agent, new_byte_length: usize) {
        agent[self].resize(new_byte_length);
    }

    /// Get temporary access to an ArrayBuffer's backing data block as a slice
    /// of bytes. The access can only be held while all JavaScript is paused.
    ///
    /// ## Safety
    ///
    /// The function itself has no safety implications, but the caller should
    /// keep in mind that if JavaScript is called into the contents of the
    /// ArrayBuffer may be rewritten or reallocated.
    pub fn as_slice(self, agent: &Agent) -> &[u8] {
        agent[self].get_data_block()
    }

    /// Get temporary exclusive access to an ArrayBuffer's backing data block
    /// as a slice of bytes. The access can only be held while all JavaScript
    /// is paused.
    ///
    /// ## Safety
    ///
    /// The function itself has no safety implications, but the caller should
    /// keep in mind that if JavaScript is called into the contents of the
    /// ArrayBuffer may be rewritten or reallocated.
    pub fn as_mut_slice(self, agent: &mut Agent) -> &mut [u8] {
        &mut *agent[self].get_data_block_mut()
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ArrayBuffer<'_> {
    type Of<'a> = ArrayBuffer<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> TryFrom<Value<'a>> for ArrayBuffer<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::ArrayBuffer(base_index) => Ok(base_index),
            _ => Err(()),
        }
    }
}

impl<'a> From<ArrayBufferIndex<'a>> for ArrayBuffer<'a> {
    fn from(value: ArrayBufferIndex<'a>) -> Self {
        ArrayBuffer(value)
    }
}

impl<'a> From<ArrayBuffer<'a>> for Object<'a> {
    fn from(value: ArrayBuffer) -> Self {
        Self::ArrayBuffer(value.unbind())
    }
}

impl<'a> From<ArrayBuffer<'a>> for Value<'a> {
    fn from(value: ArrayBuffer<'a>) -> Self {
        Self::ArrayBuffer(value)
    }
}

impl Index<ArrayBuffer<'_>> for Agent {
    type Output = ArrayBufferHeapData<'static>;

    fn index(&self, index: ArrayBuffer) -> &Self::Output {
        &self.heap.array_buffers[index]
    }
}

impl IndexMut<ArrayBuffer<'_>> for Agent {
    fn index_mut(&mut self, index: ArrayBuffer) -> &mut Self::Output {
        &mut self.heap.array_buffers[index]
    }
}

impl Index<ArrayBuffer<'_>> for Vec<Option<ArrayBufferHeapData<'static>>> {
    type Output = ArrayBufferHeapData<'static>;

    fn index(&self, index: ArrayBuffer) -> &Self::Output {
        self.get(index.get_index())
            .expect("ArrayBuffer out of bounds")
            .as_ref()
            .expect("ArrayBuffer slot empty")
    }
}

impl IndexMut<ArrayBuffer<'_>> for Vec<Option<ArrayBufferHeapData<'static>>> {
    fn index_mut(&mut self, index: ArrayBuffer) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ArrayBuffer out of bounds")
            .as_mut()
            .expect("ArrayBuffer slot empty")
    }
}

impl<'a> InternalSlots<'a> for ArrayBuffer<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::ArrayBuffer;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for ArrayBuffer<'a> {}

impl Rootable for ArrayBuffer<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::ArrayBuffer(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::ArrayBuffer(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for ArrayBuffer<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.array_buffers.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for ArrayBuffer<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.array_buffers.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<ArrayBufferHeapData<'a>, ArrayBuffer<'a>> for Heap {
    fn create(&mut self, data: ArrayBufferHeapData<'a>) -> ArrayBuffer<'a> {
        self.array_buffers.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<ArrayBufferHeapData<'static>>>();
        ArrayBuffer::from(ArrayBufferIndex::last(&self.array_buffers))
    }
}
