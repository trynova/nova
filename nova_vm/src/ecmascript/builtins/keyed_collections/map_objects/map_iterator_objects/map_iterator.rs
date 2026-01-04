// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            map::Map,
        },
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, OrdinaryObject, object_handle},
    },
    engine::context::{Bindable, bindable_handle},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, arena_vec_access,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MapIterator<'a>(BaseIndex<'a, MapIteratorHeapData<'static>>);
object_handle!(MapIterator);
arena_vec_access!(MapIterator, 'a, MapIteratorHeapData, map_iterators);

impl MapIterator<'_> {
    pub(crate) fn from_map(agent: &mut Agent, map: Map, kind: CollectionIteratorKind) -> Self {
        agent.heap.create(MapIteratorHeapData {
            object_index: None,
            map: Some(map.unbind()),
            next_index: 0,
            kind,
        })
    }
}

impl<'a> InternalSlots<'a> for MapIterator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::MapIterator;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for MapIterator<'a> {}

impl<'a> CreateHeapData<MapIteratorHeapData<'a>, MapIterator<'a>> for Heap {
    fn create(&mut self, data: MapIteratorHeapData<'a>) -> MapIterator<'a> {
        self.map_iterators.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<MapIteratorHeapData<'static>>();
        MapIterator(BaseIndex::last(&self.map_iterators))
    }
}

impl HeapMarkAndSweep for MapIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.map_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.map_iterators.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for MapIterator<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.map_iterators.shift_weak_index(self.0).map(Self)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MapIteratorHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) map: Option<Map<'a>>,
    pub(crate) next_index: usize,
    pub(crate) kind: CollectionIteratorKind,
}

bindable_handle!(MapIteratorHeapData);

impl HeapMarkAndSweep for MapIteratorHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            map,
            next_index: _,
            kind: _,
        } = self;
        object_index.mark_values(queues);
        map.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            map,
            next_index: _,
            kind: _,
        } = self;
        object_index.sweep_values(compactions);
        map.sweep_values(compactions);
    }
}
