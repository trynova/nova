// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues,
        indexes::BaseIndex,
    },
};

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
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
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

impl Index<Map<'_>> for Agent {
    type Output = MapHeapData<'static>;

    fn index(&self, index: Map) -> &Self::Output {
        &self.heap.maps[index]
    }
}

impl IndexMut<Map<'_>> for Agent {
    fn index_mut(&mut self, index: Map) -> &mut Self::Output {
        &mut self.heap.maps[index]
    }
}

impl Index<Map<'_>> for Vec<MapHeapData<'static>> {
    type Output = MapHeapData<'static>;

    fn index(&self, index: Map) -> &Self::Output {
        self.get(index.get_index()).expect("Map out of bounds")
    }
}

impl IndexMut<Map<'_>> for Vec<MapHeapData<'static>> {
    fn index_mut(&mut self, index: Map) -> &mut Self::Output {
        self.get_mut(index.get_index()).expect("Map out of bounds")
    }
}

impl<'a> CreateHeapData<MapHeapData<'a>, Map<'a>> for Heap {
    fn create(&mut self, data: MapHeapData<'a>) -> Map<'a> {
        self.maps.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<MapHeapData<'static>>();
        Map(BaseIndex::last(&self.maps))
    }
}
