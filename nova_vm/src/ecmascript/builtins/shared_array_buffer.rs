// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, SharedDataBlock, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::SharedArrayBufferRecord;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SharedArrayBuffer<'a>(BaseIndex<'a, SharedArrayBufferRecord<'static>>);

bindable_handle!(SharedArrayBuffer);

impl<'sab> SharedArrayBuffer<'sab> {
    pub(crate) const fn _def() -> Self {
        SharedArrayBuffer(BaseIndex::from_u32_index(0))
    }

    #[inline(always)]
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    fn get(self, agent: &Agent) -> &SharedArrayBufferRecord<'sab> {
        agent
            .heap
            .shared_array_buffers
            .get(self.get_index())
            .expect("Invalid SharedArrayBuffer")
    }

    fn get_mut(self, agent: &mut Agent) -> &mut SharedArrayBufferRecord<'static> {
        agent
            .heap
            .shared_array_buffers
            .get_mut(self.get_index())
            .expect("Invalid SharedArrayBuffer")
    }

    /// Returns true if the SharedArrayBuffer is growable.
    pub fn is_growable(self, agent: &Agent) -> bool {
        self.get(agent).data_block.is_growable()
    }

    /// Get the byte length of the SharedArrayBuffer.
    ///
    /// Note, if this is a growable SharedArrayBuffer then this is a
    /// synchronising operation.
    pub fn byte_length(self, agent: &Agent) -> usize {
        self.get(agent).data_block.byte_length()
    }

    /// Get the maximum byte length of the SharedArrayBuffer.
    pub fn max_byte_length(self, agent: &Agent) -> usize {
        self.get(agent).data_block.max_byte_length()
    }

    /// Set the SharedArrayBuffer's internal buffer to `data_block`.
    ///
    /// ## Safety
    ///
    /// The SharedArrayBuffer should not have had an internal buffer set.
    pub(crate) unsafe fn set_data_block(self, agent: &mut Agent, data_block: SharedDataBlock) {
        let data = self.get_mut(agent);
        debug_assert!(data.data_block.is_dangling());
        data.data_block = data_block;
    }
}

impl<'a> From<SharedArrayBuffer<'a>> for Value<'a> {
    fn from(value: SharedArrayBuffer<'a>) -> Self {
        Value::SharedArrayBuffer(value)
    }
}

impl<'a> From<SharedArrayBuffer<'a>> for Object<'a> {
    fn from(value: SharedArrayBuffer<'a>) -> Self {
        Object::SharedArrayBuffer(value)
    }
}

impl<'a> InternalSlots<'a> for SharedArrayBuffer<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SharedArrayBuffer;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).backing_object.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .backing_object
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for SharedArrayBuffer<'a> {}

impl TryFrom<HeapRootData> for SharedArrayBuffer<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::SharedArrayBuffer(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl HeapMarkAndSweep for SharedArrayBuffer<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.shared_array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.shared_array_buffers.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for SharedArrayBuffer<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .shared_array_buffers
            .shift_weak_index(self.0)
            .map(Self)
    }
}

impl<'a> CreateHeapData<SharedArrayBufferRecord<'a>, SharedArrayBuffer<'a>> for Heap {
    fn create(&mut self, data: SharedArrayBufferRecord<'a>) -> SharedArrayBuffer<'a> {
        self.shared_array_buffers.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<Option<SharedArrayBufferRecord<'static>>>();
        SharedArrayBuffer(BaseIndex::last_t(&self.shared_array_buffers))
    }
}
