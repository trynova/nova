use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{indexes::SetIndex, ObjectEntry, ObjectEntryPropertyDescriptor},
    Heap,
};

use self::data::SetHeapData;

use super::ordinary::ordinary_set_prototype_of_check_loop;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Set(pub(crate) SetIndex);

impl From<Set> for SetIndex {
    fn from(val: Set) -> Self {
        val.0
    }
}

impl From<SetIndex> for Set {
    fn from(value: SetIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Set {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Set {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Set> for Value {
    fn from(val: Set) -> Self {
        Value::Set(val.0)
    }
}

impl From<Set> for Object {
    fn from(val: Set) -> Self {
        Object::Set(val.0)
    }
}

impl Index<Set> for Agent {
    type Output = SetHeapData;

    fn index(&self, index: Set) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<Set> for Agent {
    fn index_mut(&mut self, index: Set) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<Set> for Heap {
    type Output = SetHeapData;

    fn index(&self, index: Set) -> &Self::Output {
        self.sets
            .get(index.0.into_index())
            .expect("Set out of bounds")
            .as_ref()
            .expect("Set slot empty")
    }
}

impl IndexMut<Set> for Heap {
    fn index_mut(&mut self, index: Set) -> &mut Self::Output {
        self.sets
            .get_mut(index.0.into_index())
            .expect("Set out of bounds")
            .as_mut()
            .expect("Set slot empty")
    }
}

fn create_set_base_object(agent: &mut Agent, set: Set, entries: &[ObjectEntry]) -> OrdinaryObject {
    // TODO: An issue crops up if multiple realms are in play:
    // The prototype should not be dependent on the realm we're operating in
    // but should instead be bound to the realm the object was created in.
    // We'll have to cross this bridge at a later point, likely be designating
    // a "default realm" and making non-default realms always initialize ObjectHeapData.
    let prototype = agent.current_realm().intrinsics().set_prototype();
    let object_index = agent
        .heap
        .create_object_with_prototype(prototype.into(), entries);
    agent.heap[set].object_index = Some(object_index);
    OrdinaryObject::from(object_index)
}

impl OrdinaryObjectInternalSlots for Set {
    fn internal_extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_extensible(agent)
        } else {
            true
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_set_extensible(agent, value)
        } else if !value {
            // Create base object and set inextensible
            let base = create_set_base_object(agent, self, &[]);
            base.internal_set_extensible(agent, value);
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_prototype(agent)
        } else {
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .set_prototype()
                    .into_object(),
            )
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_set_prototype(agent, prototype)
        } else {
            // Create base object and set prototype
            let base = create_set_base_object(agent, self, &[]);
            base.internal_set_prototype(agent, prototype);
        }
    }
}

impl InternalMethods for Set {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.internal_prototype(agent))
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_set_prototype_of(agent, prototype)
        } else {
            // If we're setting %Set.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().set_prototype();
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
            OrdinaryObject::from(object_index).internal_get_own_property(agent, property_key)
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
            OrdinaryObject::from(object_index).internal_has_property(agent, property_key)
        } else {
            let new_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            create_set_base_object(agent, self, &[new_entry]);
            Ok(true)
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_has_property(agent, property_key)
        } else {
            let parent = agent.current_realm().intrinsics().set_prototype();
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
            OrdinaryObject::from(object_index).internal_get(agent, property_key, receiver)
        } else {
            let parent = agent.current_realm().intrinsics().set_prototype();
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
            OrdinaryObject::from(object_index).internal_set(agent, property_key, value, receiver)
        } else {
            let prototype = agent.current_realm().intrinsics().set_prototype();
            prototype.internal_set(agent, property_key, value, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_delete(agent, property_key)
        } else {
            // Non-existing property
            Ok(true)
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent[self].object_index {
            OrdinaryObject::from(object_index).internal_own_property_keys(agent)
        } else {
            Ok(vec![])
        }
    }
}
