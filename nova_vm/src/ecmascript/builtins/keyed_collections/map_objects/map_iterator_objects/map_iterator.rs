// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

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
    engine::{
        context::{Bindable, NoGcScope},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
        indexes::MapIteratorIndex,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MapIterator<'a>(MapIteratorIndex<'a>);

impl MapIterator<'_> {
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
            map: Some(map.unbind()),
            next_index: 0,
            kind,
        })
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for MapIterator<'_> {
    type Of<'a> = MapIterator<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> IntoValue<'a> for MapIterator<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for MapIterator<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<MapIterator<'a>> for Object<'a> {
    fn from(value: MapIterator) -> Self {
        Self::MapIterator(value.unbind())
    }
}

impl<'a> From<MapIterator<'a>> for Value<'a> {
    fn from(value: MapIterator<'a>) -> Self {
        Self::MapIterator(value)
    }
}

impl<'a> TryFrom<Value<'a>> for MapIterator<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::MapIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for MapIterator<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::MapIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for MapIterator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::MapIterator;

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

impl<'a> InternalMethods<'a> for MapIterator<'a> {}

impl Index<MapIterator<'_>> for Agent {
    type Output = MapIteratorHeapData;

    fn index(&self, index: MapIterator) -> &Self::Output {
        &self.heap.map_iterators[index]
    }
}

impl IndexMut<MapIterator<'_>> for Agent {
    fn index_mut(&mut self, index: MapIterator) -> &mut Self::Output {
        &mut self.heap.map_iterators[index]
    }
}

impl Index<MapIterator<'_>> for Vec<Option<MapIteratorHeapData>> {
    type Output = MapIteratorHeapData;

    fn index(&self, index: MapIterator) -> &Self::Output {
        self.get(index.get_index())
            .expect("MapIterator out of bounds")
            .as_ref()
            .expect("Array MapIterator empty")
    }
}

impl IndexMut<MapIterator<'_>> for Vec<Option<MapIteratorHeapData>> {
    fn index_mut(&mut self, index: MapIterator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("MapIterator out of bounds")
            .as_mut()
            .expect("MapIterator slot empty")
    }
}

impl TryFrom<HeapRootData> for MapIterator<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::MapIterator(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl CreateHeapData<MapIteratorHeapData, MapIterator<'static>> for Heap {
    fn create(&mut self, data: MapIteratorHeapData) -> MapIterator<'static> {
        self.map_iterators.push(Some(data));
        MapIterator(MapIteratorIndex::last(&self.map_iterators))
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

#[derive(Debug, Clone, Copy, Default)]
pub struct MapIteratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject<'static>>,
    pub(crate) map: Option<Map<'static>>,
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
