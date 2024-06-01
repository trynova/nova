use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, MapIndex},
        CompactionLists, CreateHeapData, HeapMarkAndSweep, ObjectEntry,
        ObjectEntryPropertyDescriptor, WorkQueues,
    },
    Heap,
};

use self::data::MapHeapData;

use super::ordinary::ordinary_set_prototype_of_check_loop;

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

impl OrdinaryObjectInternalSlots for Map {
    fn internal_extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_extensible(agent)
        } else {
            true
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_extensible(agent, value)
        } else if !value {
            // Create base object and set inextensible
            let base = create_map_base_object(agent, self, &[]);
            base.internal_set_extensible(agent, value);
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_prototype(agent)
        } else {
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .map_prototype()
                    .into_object(),
            )
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_prototype(agent, prototype)
        } else {
            // Create base object and set prototype
            let base = create_map_base_object(agent, self, &[]);
            base.internal_set_prototype(agent, prototype);
        }
    }
}

impl InternalMethods for Map {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.internal_prototype(agent))
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_prototype_of(agent, prototype)
        } else {
            // If we're setting %Map.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().map_prototype();
            if prototype == Some(current.into_object()) {
                return Ok(true);
            }
            if ordinary_set_prototype_of_check_loop(agent, current.into_object(), prototype) {
                // OrdinarySetPrototypeOf 7.b.i: Setting prototype would cause a loop to occur.
                return Ok(false);
            }
            self.internal_set_prototype(agent, prototype);
            Ok(true)
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        Ok(self.internal_extensible(agent))
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        self.internal_set_extensible(agent, false);
        Ok(true)
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get_own_property(agent, property_key)
        } else {
            Ok(None)
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
        } else {
            let new_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            create_map_base_object(agent, self, &[new_entry]);
            Ok(true)
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
        } else {
            let parent = agent.current_realm().intrinsics().map_prototype();
            parent.internal_has_property(agent, property_key)
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get(agent, property_key, receiver)
        } else {
            let parent = agent.current_realm().intrinsics().map_prototype();
            parent.internal_get(agent, property_key, receiver)
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set(agent, property_key, value, receiver)
        } else {
            let prototype = agent.current_realm().intrinsics().map_prototype();
            prototype.internal_set(agent, property_key, value, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_delete(agent, property_key)
        } else {
            // Non-existing property
            Ok(true)
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_own_property_keys(agent)
        } else {
            Ok(vec![])
        }
    }
}

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
