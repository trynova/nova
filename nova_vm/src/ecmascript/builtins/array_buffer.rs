//! ## [25.1 ArrayBuffer Objects](https://tc39.es/ecma262/#sec-arraybuffer-objects)

mod abstract_operations;
mod data;
use super::ordinary::ordinary_set_prototype_of_check_loop;
use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::same_value_non_number,
        execution::{Agent, JsResult},
        types::{
            InternalMethods, Object, OrdinaryObject, OrdinaryObjectInternalSlots,
            PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{indexes::ArrayBufferIndex, GetHeapData},
};
pub use abstract_operations::*;
pub use data::ArrayBufferHeapData;
use std::ops::Deref;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ArrayBuffer(ArrayBufferIndex);

impl From<ArrayBufferIndex> for ArrayBuffer {
    fn from(value: ArrayBufferIndex) -> Self {
        ArrayBuffer(value)
    }
}

impl From<ArrayBuffer> for Object {
    fn from(value: ArrayBuffer) -> Self {
        Self::ArrayBuffer(value.0)
    }
}

impl From<ArrayBuffer> for Value {
    fn from(value: ArrayBuffer) -> Self {
        Self::ArrayBuffer(value.0)
    }
}

impl Deref for ArrayBuffer {
    type Target = ArrayBufferIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn create_array_buffer_base_object(agent: &mut Agent, array_buffer: ArrayBuffer) -> OrdinaryObject {
    // TODO: An issue crops up if multiple realms are in play:
    // The prototype should not be dependent on the realm we're operating in
    // but should instead be bound to the realm the object was created in.
    // We'll have to cross this bridge at a later point, likely be designating
    // a "default realm" and making non-default realms always initialize ObjectHeapData.
    let prototype = agent.current_realm().intrinsics().array_buffer_prototype();
    let object_index = agent.heap.create_object_with_prototype(prototype.into());
    agent.heap.get_mut(*array_buffer).object_index = Some(object_index);
    OrdinaryObject::from(object_index)
}

impl OrdinaryObjectInternalSlots for ArrayBuffer {
    fn extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).extensible(agent)
        } else {
            true
        }
    }

    fn set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).set_extensible(agent, value)
        } else {
            debug_assert!(!value);
            create_array_buffer_base_object(agent, self).set_extensible(agent, value)
        }
    }

    fn prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).prototype(agent)
        } else {
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .array_buffer_prototype()
                    .into(),
            )
        }
    }

    fn set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).set_prototype(agent, prototype)
        } else {
            // Create ArrayBuffer base object with custom prototype
            let object_index = if let Some(prototype) = prototype {
                debug_assert!(ordinary_set_prototype_of_check_loop(
                    agent,
                    prototype,
                    Some(self.into())
                ));
                agent.heap.create_object_with_prototype(prototype)
            } else {
                agent.heap.create_null_object(vec![])
            };
            agent.heap.get_mut(*self).object_index = Some(object_index);
        }
    }
}

impl InternalMethods for ArrayBuffer {
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.prototype(agent))
    }

    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).set_prototype_of(agent, prototype)
        } else {
            // If we're setting %ArrayBuffer.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().array_buffer_prototype();
            if let Some(v) = prototype {
                if same_value_non_number(agent, v, current.into()) {
                    return Ok(true);
                }
            };
            if ordinary_set_prototype_of_check_loop(agent, current.into(), prototype) {
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
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).get_own_property(agent, property_key)
        } else {
            Ok(None)
        }
    }

    fn define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> JsResult<bool> {
        create_array_buffer_base_object(agent, self).define_own_property(
            agent,
            property_key,
            property_descriptor,
        )
    }

    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).has_property(agent, property_key)
        } else {
            agent
                .current_realm()
                .intrinsics()
                .array_buffer_prototype()
                .has_property(agent, property_key)
        }
    }

    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).get(agent, property_key, receiver)
        } else {
            agent
                .current_realm()
                .intrinsics()
                .array_buffer_prototype()
                .get(agent, property_key, receiver)
        }
    }

    fn set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        create_array_buffer_base_object(agent, self).set(agent, property_key, value, receiver)
    }

    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).delete(agent, property_key)
        } else {
            // OrdinaryDelete essentially returns "didn't exist or was deleted":
            // We know properties didn't exist in this branch.
            Ok(true)
        }
    }

    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).own_property_keys(agent)
        } else {
            // OrdinaryDelete essentially returns "didn't exist or was deleted":
            // We know properties didn't exist in this branch.
            Ok(vec![])
        }
    }
}
