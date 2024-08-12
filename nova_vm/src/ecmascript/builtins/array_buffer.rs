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

pub use data::ArrayBufferHeapData;
use std::ops::{Index, IndexMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ArrayBuffer<'gen>(ArrayBufferIndex<'gen>);

impl<'gen> ArrayBuffer<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(ArrayBufferIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> IntoObject<'gen> for ArrayBuffer<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> IntoValue<'gen> for ArrayBuffer<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> From<ArrayBufferIndex<'gen>> for ArrayBuffer<'gen> {
    fn from(value: ArrayBufferIndex<'gen>) -> Self {
        ArrayBuffer(value)
    }
}

impl<'gen> From<ArrayBuffer<'gen>> for Object<'gen> {
    fn from(value: ArrayBuffer<'gen>) -> Self {
        Self::ArrayBuffer(value)
    }
}

impl<'gen> From<ArrayBuffer<'gen>> for Value<'gen> {
    fn from(value: ArrayBuffer<'gen>) -> Self {
        Self::ArrayBuffer(value)
    }
}

impl<'gen> Index<ArrayBuffer<'gen>> for Agent<'gen> {
    type Output = ArrayBufferHeapData<'gen>;

    fn index(&self, index: ArrayBuffer<'gen>) -> &Self::Output {
        &self.heap.array_buffers[index]
    }
}

impl<'gen> IndexMut<ArrayBuffer<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: ArrayBuffer<'gen>) -> &mut Self::Output {
        &mut self.heap.array_buffers[index]
    }
}

impl<'gen> Index<ArrayBuffer<'gen>> for Vec<Option<ArrayBufferHeapData<'gen>>> {
    type Output = ArrayBufferHeapData<'gen>;

    fn index(&self, index: ArrayBuffer<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("ArrayBuffer out of bounds")
            .as_ref()
            .expect("ArrayBuffer slot empty")
    }
}

impl<'gen> IndexMut<ArrayBuffer<'gen>> for Vec<Option<ArrayBufferHeapData<'gen>>> {
    fn index_mut(&mut self, index: ArrayBuffer<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ArrayBuffer out of bounds")
            .as_mut()
            .expect("ArrayBuffer slot empty")
    }
}

impl<'gen> InternalSlots<'gen> for ArrayBuffer<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::ArrayBuffer;

    #[inline(always)]
    fn get_backing_object<'b>(self, agent: &'b Agent<'gen>) -> Option<OrdinaryObject<'gen>> where 'gen: 'b {
        agent[self].object_index
    }

    fn create_backing_object<'b>(self, agent: &'b mut Agent<'gen>) -> OrdinaryObject<'gen> where 'gen: 'b {
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

impl<'gen> InternalMethods<'gen> for ArrayBuffer<'gen> {}

impl<'gen> HeapMarkAndSweep<'gen> for ArrayBuffer<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.array_buffers.shift_index(&mut self.0);
    }
}

impl<'gen> CreateHeapData<ArrayBufferHeapData<'gen>, ArrayBuffer<'gen>> for Heap<'gen> {
    fn create(&mut self, data: ArrayBufferHeapData<'gen>) -> ArrayBuffer<'gen> {
        self.array_buffers.push(Some(data));
        ArrayBuffer::from(ArrayBufferIndex::last(&self.array_buffers))
    }
}
