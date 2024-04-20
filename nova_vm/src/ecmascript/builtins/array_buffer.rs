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
    let object_index = agent
        .heap
        .create_object_with_prototype(prototype.into(), vec![]);
    agent.heap.get_mut(*array_buffer).object_index = Some(object_index);
    OrdinaryObject::from(object_index)
}

impl OrdinaryObjectInternalSlots for ArrayBuffer {
    fn internal_extensible(self, agent: &Agent) -> bool {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_extensible(agent)
        } else {
            true
        }
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_set_extensible(agent, value)
        } else {
            debug_assert!(!value);
            create_array_buffer_base_object(agent, self).internal_set_extensible(agent, value)
        }
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_prototype(agent)
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

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_set_prototype(agent, prototype)
        } else {
            // Create ArrayBuffer base object with custom prototype
            let object_index = if let Some(prototype) = prototype {
                debug_assert!(ordinary_set_prototype_of_check_loop(
                    agent,
                    prototype,
                    Some(self.into())
                ));
                agent.heap.create_object_with_prototype(prototype, vec![])
            } else {
                agent.heap.create_null_object(vec![])
            };
            agent.heap.get_mut(*self).object_index = Some(object_index);
        }
    }
}

impl InternalMethods for ArrayBuffer {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.internal_prototype(agent))
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_set_prototype_of(agent, prototype)
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
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_get_own_property(agent, property_key)
        } else {
            Ok(None)
        }
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> JsResult<bool> {
        create_array_buffer_base_object(agent, self).internal_define_own_property(
            agent,
            property_key,
            property_descriptor,
        )
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_has_property(agent, property_key)
        } else {
            agent
                .current_realm()
                .intrinsics()
                .array_buffer_prototype()
                .internal_has_property(agent, property_key)
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_get(agent, property_key, receiver)
        } else {
            agent
                .current_realm()
                .intrinsics()
                .array_buffer_prototype()
                .internal_get(agent, property_key, receiver)
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        create_array_buffer_base_object(agent, self).internal_set(
            agent,
            property_key,
            value,
            receiver,
        )
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_delete(agent, property_key)
        } else {
            // OrdinaryDelete essentially returns "didn't exist or was deleted":
            // We know properties didn't exist in this branch.
            Ok(true)
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_own_property_keys(agent)
        } else {
            // OrdinaryDelete essentially returns "didn't exist or was deleted":
            // We know properties didn't exist in this branch.
            Ok(vec![])
        }
    }
}
