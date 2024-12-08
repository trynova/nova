// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::Agent,
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, EmbedderObjectIndex},
        HeapMarkAndSweep,
    },
};

use self::data::EmbedderObjectHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EmbedderObject(pub(crate) EmbedderObjectIndex);

impl EmbedderObject {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<EmbedderObject> for EmbedderObjectIndex {
    fn from(val: EmbedderObject) -> Self {
        val.0
    }
}

impl From<EmbedderObjectIndex> for EmbedderObject {
    fn from(value: EmbedderObjectIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for EmbedderObject {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for EmbedderObject {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<EmbedderObject> for Value {
    fn from(val: EmbedderObject) -> Self {
        Value::EmbedderObject(val)
    }
}

impl From<EmbedderObject> for Object {
    fn from(val: EmbedderObject) -> Self {
        Object::EmbedderObject(val)
    }
}

impl InternalSlots for EmbedderObject {
    #[inline(always)]
    fn get_backing_object(self, _agent: &Agent) -> Option<OrdinaryObject> {
        todo!();
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject) {
        todo!();
    }

    fn create_backing_object(self, _agent: &mut Agent) -> OrdinaryObject {
        todo!();
    }
    fn internal_extensible(self, _agent: &Agent) -> bool {
        todo!();
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!();
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        todo!();
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!();
    }
}

impl InternalMethods for EmbedderObject {}

impl Index<EmbedderObject> for Agent {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObject) -> &Self::Output {
        &self.heap.embedder_objects[index]
    }
}

impl IndexMut<EmbedderObject> for Agent {
    fn index_mut(&mut self, index: EmbedderObject) -> &mut Self::Output {
        &mut self.heap.embedder_objects[index]
    }
}

impl Index<EmbedderObject> for Vec<Option<EmbedderObjectHeapData>> {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObject) -> &Self::Output {
        self.get(index.get_index())
            .expect("EmbedderObject out of bounds")
            .as_ref()
            .expect("EmbedderObject slot empty")
    }
}

impl IndexMut<EmbedderObject> for Vec<Option<EmbedderObjectHeapData>> {
    fn index_mut(&mut self, index: EmbedderObject) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("EmbedderObject out of bounds")
            .as_mut()
            .expect("EmbedderObject slot empty")
    }
}

impl HeapMarkAndSweep for EmbedderObject {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.embedder_objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.embedder_objects.shift_index(&mut self.0);
    }
}
