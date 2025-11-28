// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    Heap,
    ecmascript::{
        builtins::map::data::{MapHeapDataMut, MapHeapDataRef},
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference, PrimitiveHeap,
        PrimitiveHeapIndexable, WorkQueues, indexes::BaseIndex,
    },
};
use soavec::SoAVec;

use self::data::MapHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Map<'a>(BaseIndex<'a, MapHeapData<'static>>);

impl Map<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn get<'agent>(
        self,
        agent: &'agent impl AsRef<SoAVec<MapHeapData<'static>>>,
    ) -> MapHeapDataRef<'agent, 'static> {
        agent
            .as_ref()
            .get(self.0.into_u32_index())
            .expect("Invalid Map reference")
    }

    pub(crate) fn get_mut<'agent>(
        self,
        agent: &'agent mut impl AsMut<SoAVec<MapHeapData<'static>>>,
        arena: &impl PrimitiveHeapIndexable,
    ) -> MapHeapDataMut<'agent, 'static> {
        let mut data = agent
            .as_mut()
            .get_mut(self.0.into_u32_index())
            .expect("Invalid Map reference");
        data.rehash_if_needed_mut(arena);
        data
    }

    pub(crate) fn len(&self, agent: &mut Agent) -> u32 {
        self.get(agent).map_data.borrow().len() as u32
    }

    pub(crate) fn clear(&self, agent: &mut Agent) {
        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        self.get_mut(maps, &primitive_heap).clear();
    }
}

bindable_handle!(Map);

impl<'a> From<Map<'a>> for Value<'a> {
    fn from(value: Map<'a>) -> Self {
        Value::Map(value)
    }
}

impl<'a> From<Map<'a>> for Object<'a> {
    fn from(value: Map<'a>) -> Self {
        Object::Map(value)
    }
}

impl From<Map<'_>> for HeapRootData {
    fn from(value: Map) -> Self {
        HeapRootData::Map(value.unbind())
    }
}

impl<'a> TryFrom<Object<'a>> for Map<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::Map(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for Map<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Map(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<HeapRootData> for Map<'_> {
    type Error = ();

    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::Map(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for Map<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Map;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        assert!(
            self.get_mut(maps, &primitive_heap)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for Map<'a> {}

impl HeapMarkAndSweep for Map<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.maps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.maps.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Map<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.maps.shift_weak_index(self.0).map(Self)
    }
}

impl AsRef<SoAVec<MapHeapData<'static>>> for Agent {
    #[inline(always)]
    fn as_ref(&self) -> &SoAVec<MapHeapData<'static>> {
        &self.heap.maps
    }
}

impl AsMut<SoAVec<MapHeapData<'static>>> for Agent {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut SoAVec<MapHeapData<'static>> {
        &mut self.heap.maps
    }
}

impl<'a> CreateHeapData<MapHeapData<'a>, Map<'a>> for Heap {
    fn create(&mut self, data: MapHeapData<'a>) -> Map<'a> {
        let i = self.maps.len();
        self.maps
            .push(data.unbind())
            .expect("Failed to allocate Map");
        self.alloc_counter += core::mem::size_of::<MapHeapData<'static>>();
        Map(BaseIndex::from_u32_index(i))
    }
}
