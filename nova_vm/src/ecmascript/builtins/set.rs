use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{indexes::SetIndex, GetHeapData, ObjectEntry, ObjectEntryPropertyDescriptor},
};

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

impl OrdinaryObjectInternalSlots for Set {
    fn extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).extensible(agent)
        } else {
            true
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set_extensible(agent, value)
        } else {
            // Create base object and set inextensible
            todo!()
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).prototype(agent)
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

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set_prototype(agent, prototype)
        } else {
            // Create base object and set inextensible
            todo!()
        }
    }
}

impl InternalMethods for Set {
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.prototype(agent))
    }

    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set_prototype_of(agent, prototype)
        } else {
            // If we're setting %Set.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().map_prototype();
            if prototype == Some(current.into_object()) {
                return Ok(true);
            }
            if ordinary_set_prototype_of_check_loop(agent, current.into_object(), prototype) {
                // OrdinarySetPrototypeOf 7.b.i: Setting prototype would cause a loop to occur.
                return Ok(false);
            }
            self.set_prototype(agent, prototype);
            Ok(true)
        }
    }

    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        Ok(self.extensible(agent))
    }

    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        self.set_extensible(agent, false);
        Ok(true)
    }

    fn get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).get_own_property(agent, property_key)
        } else {
            Ok(None)
        }
    }

    fn define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).has_property(agent, property_key)
        } else {
            let prototype = agent.current_realm().intrinsics().map_prototype();
            let new_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype.into_object(), vec![new_entry]);
            agent.heap.get_mut(self.0).object_index = Some(object_index);
            Ok(true)
        }
    }

    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).has_property(agent, property_key)
        } else {
            let parent = self.get_prototype_of(agent)?;
            parent.map_or(Ok(false), |parent| parent.has_property(agent, property_key))
        }
    }

    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).get(agent, property_key, receiver)
        } else {
            let parent = self.get_prototype_of(agent)?;
            parent.map_or(Ok(Value::Undefined), |parent| {
                parent.get(agent, property_key, receiver)
            })
        }
    }

    fn set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).set(agent, property_key, value, receiver)
        } else {
            let prototype = agent.current_realm().intrinsics().map_prototype();
            prototype.set(agent, property_key, value, receiver)
        }
    }

    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).delete(agent, property_key)
        } else {
            // Non-existing property
            Ok(true)
        }
    }

    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent.heap.get(self.0).object_index {
            OrdinaryObject::from(object_index).own_property_keys(agent)
        } else {
            Ok(vec![])
        }
    }
}
