// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::FinalizationRegistryHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FinalizationRegistry<'a>(BaseIndex<'a, FinalizationRegistryHeapData<'static>>);

impl FinalizationRegistry<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

bindable_handle!(FinalizationRegistry);

impl<'a> From<FinalizationRegistry<'a>> for Value<'a> {
    fn from(value: FinalizationRegistry<'a>) -> Self {
        Value::FinalizationRegistry(value)
    }
}

impl<'a> From<FinalizationRegistry<'a>> for Object<'a> {
    fn from(value: FinalizationRegistry<'a>) -> Self {
        Object::FinalizationRegistry(value)
    }
}

impl<'a> InternalSlots<'a> for FinalizationRegistry<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::FinalizationRegistry;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for FinalizationRegistry<'a> {}

impl Index<FinalizationRegistry<'_>> for Agent {
    type Output = FinalizationRegistryHeapData<'static>;

    fn index(&self, index: FinalizationRegistry) -> &Self::Output {
        &self.heap.finalization_registrys[index]
    }
}

impl IndexMut<FinalizationRegistry<'_>> for Agent {
    fn index_mut(&mut self, index: FinalizationRegistry) -> &mut Self::Output {
        &mut self.heap.finalization_registrys[index]
    }
}

impl Index<FinalizationRegistry<'_>> for Vec<Option<FinalizationRegistryHeapData<'static>>> {
    type Output = FinalizationRegistryHeapData<'static>;

    fn index(&self, index: FinalizationRegistry) -> &Self::Output {
        self.get(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_ref()
            .expect("FinalizationRegistry slot empty")
    }
}

impl IndexMut<FinalizationRegistry<'_>> for Vec<Option<FinalizationRegistryHeapData<'static>>> {
    fn index_mut(&mut self, index: FinalizationRegistry) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_mut()
            .expect("FinalizationRegistry slot empty")
    }
}

impl Rootable for FinalizationRegistry<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::FinalizationRegistry(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::FinalizationRegistry(object) => Some(object),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<FinalizationRegistryHeapData<'a>, FinalizationRegistry<'a>> for Heap {
    fn create(&mut self, data: FinalizationRegistryHeapData<'a>) -> FinalizationRegistry<'a> {
        self.finalization_registrys.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<FinalizationRegistryHeapData<'static>>>();

        FinalizationRegistry(BaseIndex::last(&self.finalization_registrys))
    }
}

impl HeapMarkAndSweep for FinalizationRegistry<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.finalization_registrys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.finalization_registrys.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for FinalizationRegistry<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .finalization_registrys
            .shift_weak_index(self.0)
            .map(Self)
    }
}
