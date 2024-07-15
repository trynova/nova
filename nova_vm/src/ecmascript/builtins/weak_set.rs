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
        indexes::{BaseIndex, WeakSetIndex},
        CompactionLists, CreateHeapData, HeapMarkAndSweep, WorkQueues,
    },
    Heap,
};

use self::data::WeakSetHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakSet(pub(crate) WeakSetIndex);

impl WeakSet {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<WeakSet> for WeakSetIndex {
    fn from(val: WeakSet) -> Self {
        val.0
    }
}

impl From<WeakSetIndex> for WeakSet {
    fn from(value: WeakSetIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for WeakSet {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for WeakSet {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<WeakSet> for Value {
    fn from(val: WeakSet) -> Self {
        Value::WeakSet(val)
    }
}

impl From<WeakSet> for Object {
    fn from(val: WeakSet) -> Self {
        Object::WeakSet(val)
    }
}

impl InternalSlots for WeakSet {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakSet;

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

impl InternalMethods for WeakSet {}

impl HeapMarkAndSweep for WeakSetHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}

impl Index<WeakSet> for Agent {
    type Output = WeakSetHeapData;

    fn index(&self, index: WeakSet) -> &Self::Output {
        &self.heap.weak_sets[index]
    }
}

impl IndexMut<WeakSet> for Agent {
    fn index_mut(&mut self, index: WeakSet) -> &mut Self::Output {
        &mut self.heap.weak_sets[index]
    }
}

impl Index<WeakSet> for Vec<Option<WeakSetHeapData>> {
    type Output = WeakSetHeapData;

    fn index(&self, index: WeakSet) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakSet out of bounds")
            .as_ref()
            .expect("WeakSet slot empty")
    }
}

impl IndexMut<WeakSet> for Vec<Option<WeakSetHeapData>> {
    fn index_mut(&mut self, index: WeakSet) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakSet out of bounds")
            .as_mut()
            .expect("WeakSet slot empty")
    }
}

impl CreateHeapData<WeakSetHeapData, WeakSet> for Heap {
    fn create(&mut self, data: WeakSetHeapData) -> WeakSet {
        self.weak_sets.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakSet(WeakSetIndex::last(&self.weak_sets))
    }
}

impl HeapMarkAndSweep for WeakSet {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.weak_sets.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.weak_sets.shift_index(&mut self.0);
    }
}
