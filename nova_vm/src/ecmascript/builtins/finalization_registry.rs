// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
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
pub struct FinalizationRegistry(pub(crate) FinalizationRegistryIndex);

impl FinalizationRegistry {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<FinalizationRegistry> for FinalizationRegistryIndex {
    fn from(val: FinalizationRegistry) -> Self {
        val.0
    }
}

impl From<FinalizationRegistryIndex> for FinalizationRegistry {
    fn from(value: FinalizationRegistryIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for FinalizationRegistry {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for FinalizationRegistry {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<FinalizationRegistry> for Value {
    fn from(val: FinalizationRegistry) -> Self {
        Value::FinalizationRegistry(val)
    }
}

impl From<FinalizationRegistry> for Object {
    fn from(val: FinalizationRegistry) -> Self {
        Object::FinalizationRegistry(val)
    }
}

impl InternalSlots for FinalizationRegistry {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::FinalizationRegistry;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl InternalMethods for FinalizationRegistry {}

impl Index<FinalizationRegistry> for Agent {
    type Output = FinalizationRegistryHeapData;

    fn index(&self, index: FinalizationRegistry) -> &Self::Output {
        &self.heap.finalization_registrys[index]
    }
}

impl IndexMut<FinalizationRegistry> for Agent {
    fn index_mut(&mut self, index: FinalizationRegistry) -> &mut Self::Output {
        &mut self.heap.finalization_registrys[index]
    }
}

impl Index<FinalizationRegistry> for Vec<Option<FinalizationRegistryHeapData>> {
    type Output = FinalizationRegistryHeapData;

    fn index(&self, index: FinalizationRegistry) -> &Self::Output {
        self.get(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_ref()
            .expect("FinalizationRegistry slot empty")
    }
}

impl IndexMut<FinalizationRegistry> for Vec<Option<FinalizationRegistryHeapData>> {
    fn index_mut(&mut self, index: FinalizationRegistry) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_mut()
            .expect("FinalizationRegistry slot empty")
    }
}

impl CreateHeapData<FinalizationRegistryHeapData, FinalizationRegistry> for Heap {
    fn create(&mut self, data: FinalizationRegistryHeapData) -> FinalizationRegistry {
        self.finalization_registrys.push(Some(data));
        FinalizationRegistry(FinalizationRegistryIndex::last(
            &self.finalization_registrys,
        ))
    }
}

impl HeapMarkAndSweep for FinalizationRegistry {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.finalization_registrys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.finalization_registrys.shift_index(&mut self.0);
    }
}
