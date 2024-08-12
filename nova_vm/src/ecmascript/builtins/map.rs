// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, MapIndex},
        CompactionLists, CreateHeapData, HeapMarkAndSweep, ObjectEntry, WorkQueues,
    },
    Heap,
};

use self::data::MapHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Map<'gen>(pub(crate) MapIndex<'gen>);

impl<'gen> Map<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<Map<'gen>> for MapIndex<'gen> {
    fn from(val: Map<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<MapIndex<'gen>> for Map<'gen> {
    fn from(value: MapIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for Map<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for Map<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<Map<'gen>> for Value<'gen> {
    fn from(val: Map<'gen>) -> Self {
        Value::Map(val)
    }
}

impl<'gen> From<Map<'gen>> for Object<'gen> {
    fn from(val: Map<'gen>) -> Self {
        Object::Map(val)
    }
}

impl<'gen> TryFrom<Object<'gen>> for Map<'gen> {
    type Error = ();

    fn try_from(value: Object<'gen>) -> Result<Self, Self::Error> {
        match value {
            Object::Map(data) => Ok(data),
            _ => Err(()),
        }
    }
}

fn create_map_base_object<'gen>(agent: &mut Agent<'gen>, map: Map<'gen>, entries: &[ObjectEntry<'gen>]) -> OrdinaryObject<'gen> {
    // TODO: An issue crops up if multiple realms are in play:
    // The prototype should not be dependent on the realm we're operating in
    // but should instead be bound to the realm the object was created in.
    // We'll have to cross this bridge at a later point, likely be designating
    // a "default realm" and making non-default realms always initialize ObjectHeapData.
    let prototype = agent.current_realm().intrinsics().map_prototype();
    let object_index = agent
        .heap
        .create_object_with_prototype(prototype.into(), entries);
    agent[map].object_index = Some(object_index);
    object_index
}

impl<'gen> InternalSlots<'gen> for Map<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Map;

    #[inline(always)]
    fn get_backing_object<'b>(self, agent: &'b Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> where 'gen: 'b {
        agent[self].object_index
    }

    fn create_backing_object<'b>(self, agent: &'b mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> where 'gen: 'b {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl<'gen> InternalMethods<'gen> for Map<'gen> {}

impl<'gen> HeapMarkAndSweep<'gen> for Map<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.maps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.maps.shift_index(&mut self.0);
    }
}

impl<'gen> Index<Map<'gen>> for Agent<'gen> {
    type Output = MapHeapData<'gen>;

    fn index(&self, index: Map<'gen>) -> &Self::Output {
        &self.heap.maps[index]
    }
}

impl<'gen> IndexMut<Map<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: Map<'gen>) -> &mut Self::Output {
        &mut self.heap.maps[index]
    }
}

impl<'gen> Index<Map<'gen>> for Vec<Option<MapHeapData<'gen>>> {
    type Output = MapHeapData<'gen>;

    fn index(&self, index: Map<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Map out of bounds")
            .as_ref()
            .expect("Map slot empty")
    }
}

impl<'gen> IndexMut<Map<'gen>> for Vec<Option<MapHeapData<'gen>>> {
    fn index_mut(&mut self, index: Map<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Map out of bounds")
            .as_mut()
            .expect("Map slot empty")
    }
}

impl<'gen> CreateHeapData<MapHeapData<'gen>, Map<'gen>> for Heap<'gen> {
    fn create(&mut self, data: MapHeapData<'gen>) -> Map<'gen> {
        self.maps.push(Some(data));
        Map(MapIndex::last(&self.maps))
    }
}
