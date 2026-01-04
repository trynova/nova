// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, OrdinaryObject, object_handle},
    },
    engine::context::Bindable,
    heap::{
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference, PrimitiveHeap,
        WorkQueues, arena_vec_access,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

use self::data::{MapHeapData, MapHeapDataMut, MapHeapDataRef};
use soavec::SoAVec;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Map<'a>(BaseIndex<'a, MapHeapData<'static>>);
object_handle!(Map);
arena_vec_access!(
    soa:
    Map,
    'a,
    MapHeapData,
    maps,
    MapHeapDataRef,
    MapHeapDataMut
);

impl<'gc> Map<'gc> {
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
        maps.get(self.0.get_index_u32())
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
                maps.get_mut(self.0.get_index_u32())
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
        Map(BaseIndex::from_index_u32(i))
    }
}
