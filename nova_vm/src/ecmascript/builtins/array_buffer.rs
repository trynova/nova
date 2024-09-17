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
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::ArrayBufferIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

pub(crate) use abstract_operations::allocate_array_buffer;
pub use data::ArrayBufferHeapData;
use std::ops::{Index, IndexMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ArrayBuffer(ArrayBufferIndex);

impl ArrayBuffer {
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
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
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
