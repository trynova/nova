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
        indexes::{BaseIndex, FinalizationRegistryIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::FinalizationRegistryHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FinalizationRegistry<'gen>(pub(crate) FinalizationRegistryIndex<'gen>);

impl<'gen> FinalizationRegistry<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<FinalizationRegistry<'gen>> for FinalizationRegistryIndex<'gen> {
    fn from(val: FinalizationRegistry<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<FinalizationRegistryIndex<'gen>> for FinalizationRegistry<'gen> {
    fn from(value: FinalizationRegistryIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for FinalizationRegistry<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for FinalizationRegistry<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<FinalizationRegistry<'gen>> for Value<'gen> {
    fn from(val: FinalizationRegistry<'gen>) -> Self {
        Value::FinalizationRegistry(val)
    }
}

impl<'gen> From<FinalizationRegistry<'gen>> for Object<'gen> {
    fn from(val: FinalizationRegistry<'gen>) -> Self {
        Object::FinalizationRegistry(val)
    }
}

impl<'gen> InternalSlots<'gen> for FinalizationRegistry<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::FinalizationRegistry;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
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

impl<'gen> InternalMethods<'gen> for FinalizationRegistry<'gen> {}

impl<'gen> Index<FinalizationRegistry<'gen>> for Agent<'gen> {
    type Output = FinalizationRegistryHeapData<'gen>;

    fn index(&self, index: FinalizationRegistry<'gen>) -> &Self::Output {
        &self.heap.finalization_registrys[index]
    }
}

impl<'gen> IndexMut<FinalizationRegistry<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: FinalizationRegistry<'gen>) -> &mut Self::Output {
        &mut self.heap.finalization_registrys[index]
    }
}

impl<'gen> Index<FinalizationRegistry<'gen>> for Vec<Option<FinalizationRegistryHeapData<'gen>>> {
    type Output = FinalizationRegistryHeapData<'gen>;

    fn index(&self, index: FinalizationRegistry<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_ref()
            .expect("FinalizationRegistry slot empty")
    }
}

impl<'gen> IndexMut<FinalizationRegistry<'gen>> for Vec<Option<FinalizationRegistryHeapData<'gen>>> {
    fn index_mut(&mut self, index: FinalizationRegistry<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_mut()
            .expect("FinalizationRegistry slot empty")
    }
}

impl<'gen> CreateHeapData<FinalizationRegistryHeapData<'gen>, FinalizationRegistry<'gen>> for Heap<'gen> {
    fn create(&mut self, data: FinalizationRegistryHeapData<'gen>) -> FinalizationRegistry<'gen> {
        self.finalization_registrys.push(Some(data));
        FinalizationRegistry(FinalizationRegistryIndex::last(
            &self.finalization_registrys,
        ))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for FinalizationRegistry<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.finalization_registrys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.finalization_registrys.shift_index(&mut self.0);
    }
}
