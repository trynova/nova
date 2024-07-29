// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData, Value,
        },
    },
    heap::{
        indexes::SharedArrayBufferIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

use self::data::SharedArrayBufferHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SharedArrayBuffer<'gen>(pub(crate) SharedArrayBufferIndex<'gen>);

impl<'gen> SharedArrayBuffer<'gen> {
    pub(crate) const fn _def() -> Self {
        SharedArrayBuffer(SharedArrayBufferIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<SharedArrayBuffer<'gen>> for SharedArrayBufferIndex<'gen> {
    fn from(val: SharedArrayBuffer<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<SharedArrayBufferIndex<'gen>> for SharedArrayBuffer<'gen> {
    fn from(value: SharedArrayBufferIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for SharedArrayBuffer<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for SharedArrayBuffer<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<SharedArrayBuffer<'gen>> for Value<'gen> {
    fn from(val: SharedArrayBuffer<'gen>) -> Self {
        Value::SharedArrayBuffer(val)
    }
}

impl<'gen> From<SharedArrayBuffer<'gen>> for Object<'gen> {
    fn from(val: SharedArrayBuffer<'gen>) -> Self {
        Object::SharedArrayBuffer(val)
    }
}

impl<'gen> Index<SharedArrayBuffer<'gen>> for Agent<'gen> {
    type Output = SharedArrayBufferHeapData<'gen>;

    fn index(&self, index: SharedArrayBuffer<'gen>) -> &Self::Output {
        &self.heap.shared_array_buffers[index]
    }
}

impl<'gen> IndexMut<SharedArrayBuffer<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: SharedArrayBuffer<'gen>) -> &mut Self::Output {
        &mut self.heap.shared_array_buffers[index]
    }
}

impl<'gen> Index<SharedArrayBuffer<'gen>> for Vec<Option<SharedArrayBufferHeapData<'gen>>> {
    type Output = SharedArrayBufferHeapData<'gen>;

    fn index(&self, index: SharedArrayBuffer) -> &Self::Output {
        self.get(index.get_index())
            .expect("SharedArrayBuffer out of bounds")
            .as_ref()
            .expect("SharedArrayBuffer slot empty")
    }
}

impl<'gen> IndexMut<SharedArrayBuffer<'gen>> for Vec<Option<SharedArrayBufferHeapData<'gen>>> {
    fn index_mut(&mut self, index: SharedArrayBuffer<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("SharedArrayBuffer out of bounds")
            .as_mut()
            .expect("SharedArrayBuffer slot empty")
    }
}

impl<'gen> InternalSlots<'gen> for SharedArrayBuffer<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SharedArrayBuffer;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
        debug_assert!(self.get_backing_object(agent).is_none());
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

impl<'gen> InternalMethods<'gen> for SharedArrayBuffer<'gen> {}

impl<'gen> HeapMarkAndSweep<'gen> for SharedArrayBuffer<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.shared_array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.shared_array_buffers.shift_index(&mut self.0);
    }
}

impl<'gen> CreateHeapData<SharedArrayBufferHeapData<'gen>, SharedArrayBuffer<'gen>> for Heap<'gen> {
    fn create(&mut self, data: SharedArrayBufferHeapData<'gen>) -> SharedArrayBuffer<'gen> {
        self.shared_array_buffers.push(Some(data));
        SharedArrayBuffer(SharedArrayBufferIndex::last(&self.shared_array_buffers))
    }
}
