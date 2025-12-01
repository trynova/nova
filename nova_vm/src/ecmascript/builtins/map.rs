// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference, PrimitiveHeap,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::{MapHeapData, MapHeapDataMut, MapHeapDataRef};
use soavec::SoAVec;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Map<'a>(BaseIndex<'a, MapHeapData<'static>>);

impl<'gc> Map<'gc> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    pub(crate) fn get<'a>(self, agent: &'a Agent) -> MapHeapDataRef<'a, 'gc> {
        self.get_direct(&agent.heap.maps)
    }

    #[inline(always)]
    pub(crate) fn get_mut<'a>(self, agent: &'a mut Agent) -> MapHeapDataMut<'a, 'gc> {
        let Heap {
            bigints,
            numbers,
            strings,
            maps,
            ..
        } = &mut agent.heap;
        let primitive_heap = PrimitiveHeap::new(bigints, numbers, strings);
        let mut data = self.get_direct_mut(maps);
        data.rehash_if_needed_mut(&primitive_heap);
        data
    }

    #[inline(always)]
    pub(crate) fn get_direct<'a>(
        self,
        maps: &'a SoAVec<MapHeapData<'static>>,
    ) -> MapHeapDataRef<'a, 'gc> {
        maps.get(self.0.into_u32_index())
            .expect("Invalid Map reference")
    }

    #[inline(always)]
    pub(crate) fn get_direct_mut<'a>(
        self,
        maps: &'a mut SoAVec<MapHeapData<'static>>,
    ) -> MapHeapDataMut<'a, 'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<MapHeapDataMut<'a, 'static>, MapHeapDataMut<'a, 'gc>>(
                maps.get_mut(self.0.into_u32_index())
                    .expect("Invalid Map reference"),
            )
        }
    }

    pub(crate) fn len(&self, agent: &mut Agent) -> u32 {
        self.get(agent).size()
    }

    pub(crate) fn clear(&self, agent: &mut Agent) {
        self.get_mut(agent).clear();
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
        assert!(
            self.get_mut(agent)
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
