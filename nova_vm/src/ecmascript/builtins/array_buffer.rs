//! ## [25.1 ArrayBuffer Objects](https://tc39.es/ecma262/#sec-arraybuffer-objects)

mod abstract_operations;
mod data;
use super::ordinary::ordinary_set_prototype_of_check_loop;
use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::same_value_non_number,
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{
        indexes::ArrayBufferIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

pub use data::ArrayBufferHeapData;
use std::ops::{Index, IndexMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ArrayBuffer(ArrayBufferIndex);

impl ArrayBuffer {
    pub(crate) const fn _def() -> Self {
        Self(ArrayBufferIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoObject for ArrayBuffer {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl IntoValue for ArrayBuffer {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl From<ArrayBufferIndex> for ArrayBuffer {
    fn from(value: ArrayBufferIndex) -> Self {
        ArrayBuffer(value)
    }
}

impl From<ArrayBuffer> for Object {
    fn from(value: ArrayBuffer) -> Self {
        Self::ArrayBuffer(value)
    }
}

impl From<ArrayBuffer> for Value {
    fn from(value: ArrayBuffer) -> Self {
        Self::ArrayBuffer(value)
    }
}

impl Index<ArrayBuffer> for Agent {
    type Output = ArrayBufferHeapData;

    fn index(&self, index: ArrayBuffer) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<ArrayBuffer> for Agent {
    fn index_mut(&mut self, index: ArrayBuffer) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<ArrayBuffer> for Heap {
    type Output = ArrayBufferHeapData;

    fn index(&self, index: ArrayBuffer) -> &Self::Output {
        self.array_buffers
            .get(index.0.into_index())
            .expect("ArrayBuffer out of bounds")
            .as_ref()
            .expect("ArrayBuffer slot empty")
    }
}

impl IndexMut<ArrayBuffer> for Heap {
    fn index_mut(&mut self, index: ArrayBuffer) -> &mut Self::Output {
        self.array_buffers
            .get_mut(index.0.into_index())
            .expect("ArrayBuffer out of bounds")
            .as_mut()
            .expect("ArrayBuffer slot empty")
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
        .create_object_with_prototype(prototype.into(), &[]);
    agent[array_buffer].object_index = Some(object_index);
    object_index
}

impl OrdinaryObjectInternalSlots for ArrayBuffer {
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
        } else {
            debug_assert!(!value);
            create_array_buffer_base_object(agent, self).internal_set_extensible(agent, value)
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
                    .array_buffer_prototype()
                    .into(),
            )
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_prototype(agent, prototype)
        } else {
            // Create ArrayBuffer base object with custom prototype
            let object_index = if let Some(prototype) = prototype {
                debug_assert!(ordinary_set_prototype_of_check_loop(
                    agent,
                    prototype,
                    Some(self.into())
                ));
                agent.heap.create_object_with_prototype(prototype, &[])
            } else {
                agent.heap.create_null_object(&[])
            };
            agent[self].object_index = Some(object_index);
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
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_prototype_of(agent, prototype)
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
        property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> JsResult<bool> {
        create_array_buffer_base_object(agent, self).internal_define_own_property(
            agent,
            property_key,
            property_descriptor,
        )
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
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
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_get(agent, property_key, receiver)
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
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_delete(agent, property_key)
        } else {
            // OrdinaryDelete essentially returns "didn't exist or was deleted":
            // We know properties didn't exist in this branch.
            Ok(true)
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_own_property_keys(agent)
        } else {
            // OrdinaryDelete essentially returns "didn't exist or was deleted":
            // We know properties didn't exist in this branch.
            Ok(vec![])
        }
    }
}

impl HeapMarkAndSweep for ArrayBuffer {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = ArrayBufferIndex::from_u32(
            self_index - compactions.array_buffers.get_shift_for_index(self_index),
        );
    }
}

impl CreateHeapData<ArrayBufferHeapData, ArrayBuffer> for Heap {
    fn create(&mut self, data: ArrayBufferHeapData) -> ArrayBuffer {
        self.array_buffers.push(Some(data));
        ArrayBuffer::from(ArrayBufferIndex::last(&self.array_buffers))
    }
}
