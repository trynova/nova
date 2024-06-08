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
pub struct Map(pub(crate) MapIndex);

impl Map {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<Map> for MapIndex {
    fn from(val: Map) -> Self {
        val.0
    }
}

impl From<MapIndex> for Map {
    fn from(value: MapIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Map {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Map {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Map> for Value {
    fn from(val: Map) -> Self {
        Value::Map(val)
    }
}

impl From<Map> for Object {
    fn from(val: Map) -> Self {
        Object::Map(val)
    }
}

fn create_map_base_object(agent: &mut Agent, map: Map, entries: &[ObjectEntry]) -> OrdinaryObject {
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

impl InternalSlots for Map {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Map;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
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

impl InternalMethods for Map {}

impl HeapMarkAndSweep for Map {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.maps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = MapIndex::from_u32(self_index - compactions.maps.get_shift_for_index(self_index));
    }
}

impl Index<Map> for Agent {
    type Output = MapHeapData;

    fn index(&self, index: Map) -> &Self::Output {
        self.heap
            .maps
            .get(index.get_index())
            .expect("Map out of bounds")
            .as_ref()
            .expect("Map slot empty")
    }
}

impl IndexMut<Map> for Agent {
    fn index_mut(&mut self, index: Map) -> &mut Self::Output {
        self.heap
            .maps
            .get_mut(index.get_index())
            .expect("Map out of bounds")
            .as_mut()
            .expect("Map slot empty")
    }
}

impl CreateHeapData<MapHeapData, Map> for Heap {
    fn create(&mut self, data: MapHeapData) -> Map {
        self.maps.push(Some(data));
        Map(MapIndex::last(&self.maps))
    }
}
