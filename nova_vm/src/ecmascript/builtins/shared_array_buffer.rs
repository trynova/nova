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
pub struct SharedArrayBuffer(pub(crate) SharedArrayBufferIndex);

impl SharedArrayBuffer {
    pub(crate) const fn _def() -> Self {
        SharedArrayBuffer(SharedArrayBufferIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<SharedArrayBuffer> for SharedArrayBufferIndex {
    fn from(val: SharedArrayBuffer) -> Self {
        val.0
    }
}

impl From<SharedArrayBufferIndex> for SharedArrayBuffer {
    fn from(value: SharedArrayBufferIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for SharedArrayBuffer {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for SharedArrayBuffer {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<SharedArrayBuffer> for Value {
    fn from(val: SharedArrayBuffer) -> Self {
        Value::SharedArrayBuffer(val)
    }
}

impl From<SharedArrayBuffer> for Object {
    fn from(val: SharedArrayBuffer) -> Self {
        Object::SharedArrayBuffer(val)
    }
}

impl Index<SharedArrayBuffer> for Agent {
    type Output = SharedArrayBufferHeapData;

    fn index(&self, index: SharedArrayBuffer) -> &Self::Output {
        &self.heap.shared_array_buffers[index]
    }
}

impl IndexMut<SharedArrayBuffer> for Agent {
    fn index_mut(&mut self, index: SharedArrayBuffer) -> &mut Self::Output {
        &mut self.heap.shared_array_buffers[index]
    }
}

impl Index<SharedArrayBuffer> for Vec<Option<SharedArrayBufferHeapData>> {
    type Output = SharedArrayBufferHeapData;

    fn index(&self, index: SharedArrayBuffer) -> &Self::Output {
        self.get(index.get_index())
            .expect("SharedArrayBuffer out of bounds")
            .as_ref()
            .expect("SharedArrayBuffer slot empty")
    }
}

impl IndexMut<SharedArrayBuffer> for Vec<Option<SharedArrayBufferHeapData>> {
    fn index_mut(&mut self, index: SharedArrayBuffer) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("SharedArrayBuffer out of bounds")
            .as_mut()
            .expect("SharedArrayBuffer slot empty")
    }
}

impl InternalSlots for SharedArrayBuffer {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SharedArrayBuffer;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
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

impl InternalMethods for SharedArrayBuffer {}

impl HeapMarkAndSweep for SharedArrayBuffer {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.shared_array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = SharedArrayBufferIndex::from_u32(
            self_index
                - compactions
                    .shared_array_buffers
                    .get_shift_for_index(self_index),
        );
    }
}

impl CreateHeapData<SharedArrayBufferHeapData, SharedArrayBuffer> for Heap {
    fn create(&mut self, data: SharedArrayBufferHeapData) -> SharedArrayBuffer {
        self.shared_array_buffers.push(Some(data));
        SharedArrayBuffer(SharedArrayBufferIndex::last(&self.shared_array_buffers))
    }
}
