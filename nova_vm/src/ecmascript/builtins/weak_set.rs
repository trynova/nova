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
pub struct WeakSet<'gen>(pub(crate) WeakSetIndex<'gen>);

impl WeakSet<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<WeakSet<'gen>> for WeakSetIndex<'gen> {
    fn from(val: WeakSet<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<WeakSetIndex<'gen>> for WeakSet<'gen> {
    fn from(value: WeakSetIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for WeakSet<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for WeakSet<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<WeakSet<'gen>> for Value<'gen> {
    fn from(val: WeakSet<'gen>) -> Self {
        Value::WeakSet(val)
    }
}

impl<'gen> From<WeakSet<'gen>> for Object<'gen> {
    fn from(val: WeakSet<'gen>) -> Self {
        Object::WeakSet(val)
    }
}

impl<'gen> InternalSlots<'gen> for WeakSet<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakSet;

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

impl<'gen> InternalMethods<'gen> for WeakSet<'gen> {}

impl<'gen> HeapMarkAndSweep<'gen> for WeakSetHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}

impl<'gen> Index<WeakSet<'gen>> for Agent<'gen> {
    type Output = WeakSetHeapData<'gen>;

    fn index(&self, index: WeakSet<'gen>) -> &Self::Output {
        &self.heap.weak_sets[index]
    }
}

impl<'gen> IndexMut<WeakSet<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: WeakSet<'gen>) -> &mut Self::Output {
        &mut self.heap.weak_sets[index]
    }
}

impl<'gen> Index<WeakSet<'gen>> for Vec<Option<WeakSetHeapData<'gen>>> {
    type Output = WeakSetHeapData<'gen>;

    fn index(&self, index: WeakSet<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakSet out of bounds")
            .as_ref()
            .expect("WeakSet slot empty")
    }
}

impl<'gen> IndexMut<WeakSet<'gen>> for Vec<Option<WeakSetHeapData<'gen>>> {
    fn index_mut(&mut self, index: WeakSet<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakSet out of bounds")
            .as_mut()
            .expect("WeakSet slot empty")
    }
}

impl<'gen> CreateHeapData<WeakSetHeapData<'gen>, WeakSet<'gen>> for Heap<'gen> {
    fn create(&mut self, data: WeakSetHeapData<'gen>) -> WeakSet<'gen> {
        self.weak_sets.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakSet(WeakSetIndex::last(&self.weak_sets))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for WeakSet<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.weak_sets.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.weak_sets.shift_index(&mut self.0);
    }
}
