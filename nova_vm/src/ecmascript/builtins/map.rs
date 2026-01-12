// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use hashbrown::HashTable;
use soavec::SoAVec;

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, OrdinaryObject, Value, object_handle},
    },
    engine::context::Bindable,
    heap::{
        ArenaAccessSoA, ArenaAccessSoAMut, CompactionLists, CreateHeapData, HeapMarkAndSweep,
        HeapSweepWeakReference, PrimitiveHeapAccess, WorkQueues, arena_vec_access,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

pub(crate) use self::data::MapHeapData;
use self::data::MapHeapDataMut;

mod data;

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
    pub(crate) fn len(self, agent: &mut Agent) -> u32 {
        self.get(agent).size()
    }

    pub(crate) fn entries_len(self, agent: &Agent) -> u32 {
        self.get(agent).entries_len()
    }

    pub(crate) fn clear(self, agent: &mut Agent) {
        self.get_mut(agent).clear();
    }

    pub(crate) fn get_entries(
        self,
        agent: &Agent,
    ) -> (&[Option<Value<'gc>>], &[Option<Value<'gc>>]) {
        let data = self.get(agent);
        (data.keys, data.values)
    }

    pub(crate) fn get_map_data<'soa>(
        self,
        maps: &'soa mut SoAVec<MapHeapData<'static>>,
        arena: &impl PrimitiveHeapAccess,
    ) -> (
        &'soa HashTable<u32>,
        &'soa [Option<Value<'gc>>],
        &'soa [Option<Value<'gc>>],
    ) {
        let mut data = self.get_mut(maps);
        data.rehash_if_needed_mut(arena);
        let MapHeapDataMut {
            map_data,
            keys,
            values,
            ..
        } = data;
        (map_data.get_mut(), keys, values)
    }

    pub(crate) fn get_map_data_mut<'soa>(
        self,
        maps: &'soa mut SoAVec<MapHeapData<'static>>,
        arena: &impl PrimitiveHeapAccess,
    ) -> (
        &'soa mut HashTable<u32>,
        &'soa mut Vec<Option<Value<'static>>>,
        &'soa mut Vec<Option<Value<'static>>>,
    ) {
        let mut data = self.get_mut(maps);
        data.rehash_if_needed_mut(arena);
        let MapHeapDataMut {
            map_data,
            keys,
            values,
            ..
        } = data;
        (map_data.get_mut(), keys, values)
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
