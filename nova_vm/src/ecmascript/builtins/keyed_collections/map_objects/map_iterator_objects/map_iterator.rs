// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::{
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            map::Map,
        },
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::MapIteratorIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MapIterator(MapIteratorIndex);

impl MapIterator {
    /// # Do not use this
    /// This is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        Self(MapIteratorIndex::from_u32_index(0))
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn from_map(agent: &mut Agent, map: Map, kind: CollectionIteratorKind) -> Self {
        agent.heap.create(MapIteratorHeapData {
            object_index: None,
            map: Some(map),
            next_index: 0,
            kind,
        })
    }
}

impl IntoValue for MapIterator {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for MapIterator {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<MapIteratorIndex> for MapIterator {
    fn from(value: MapIteratorIndex) -> Self {
        MapIterator(value)
    }
}

impl From<MapIterator> for Object {
    fn from(value: MapIterator) -> Self {
        Self::MapIterator(value)
    }
}

impl From<MapIterator> for Value {
    fn from(value: MapIterator) -> Self {
        Self::MapIterator(value)
    }
}

impl TryFrom<Value> for MapIterator {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::MapIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for MapIterator {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::MapIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl InternalSlots for MapIterator {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::MapIterator;

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl InternalMethods for MapIterator {}

impl Index<MapIterator> for Agent {
    type Output = MapIteratorHeapData;

    fn index(&self, index: MapIterator) -> &Self::Output {
        &self.heap.map_iterators[index]
    }
}

impl IndexMut<MapIterator> for Agent {
    fn index_mut(&mut self, index: MapIterator) -> &mut Self::Output {
        &mut self.heap.map_iterators[index]
    }
}

impl Index<MapIterator> for Vec<Option<MapIteratorHeapData>> {
    type Output = MapIteratorHeapData;

    fn index(&self, index: MapIterator) -> &Self::Output {
        self.get(index.get_index())
            .expect("MapIterator out of bounds")
            .as_ref()
            .expect("Array MapIterator empty")
    }
}

impl IndexMut<MapIterator> for Vec<Option<MapIteratorHeapData>> {
    fn index_mut(&mut self, index: MapIterator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("MapIterator out of bounds")
            .as_mut()
            .expect("MapIterator slot empty")
    }
}

impl CreateHeapData<MapIteratorHeapData, MapIterator> for Heap {
    fn create(&mut self, data: MapIteratorHeapData) -> MapIterator {
        self.map_iterators.push(Some(data));
        MapIterator::from(MapIteratorIndex::last(&self.map_iterators))
    }
}

impl HeapMarkAndSweep for MapIterator {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.map_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.map_iterators.shift_index(&mut self.0);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MapIteratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) map: Option<Map>,
    pub(crate) next_index: usize,
    pub(crate) kind: CollectionIteratorKind,
}

impl HeapMarkAndSweep for MapIteratorHeapData {
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
