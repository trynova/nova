//! ### 10.4.2 Array Exotic Objects
//!
//! https://tc39.es/ecma262/#sec-array-exotic-objects

pub(crate) mod abstract_operations;
mod data;

use std::ops::{Deref, Index, IndexMut};

use super::{array_set_length, ordinary::ordinary_define_own_property};
use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        element_array::ElementsVector, indexes::ArrayIndex, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

pub use data::{ArrayHeapData, SealableElementsVector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array(ArrayIndex);

impl Array {
    /// # Do not use this
    /// This is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        Self(ArrayIndex::from_u32_index(0))
    }
    pub fn len(&self, agent: &Agent) -> u32 {
        agent[*self].elements.len()
    }
}

impl IntoValue for Array {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Array {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<ArrayIndex> for Array {
    fn from(value: ArrayIndex) -> Self {
        Array(value)
    }
}

impl From<Array> for Object {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

impl From<Array> for Value {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

impl TryFrom<Value> for Array {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for Array {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::Array(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl Deref for Array {
    type Target = ArrayIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl OrdinaryObjectInternalSlots for Array {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Array;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
        let prototype = Some(
            agent
                .current_realm()
                .intrinsics()
                .array_prototype()
                .into_object(),
        );
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype,
            keys: ElementsVector::default(),
            values: ElementsVector::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }

    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        agent[self].elements.len_writable = value;
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_extensible(agent, value)
        } else if !value {
            self.create_backing_object(agent)
                .internal_set_extensible(agent, value);
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_set_prototype(agent, prototype)
        } else {
            // 1. Let current be O.[[Prototype]].
            let current = agent.current_realm().intrinsics().array_prototype();
            if prototype == Some(current.into_object()) {
                return;
            }
            // Create array base object with custom prototype
            self.create_backing_object(agent)
                .internal_set_prototype(agent, prototype);
        }
    }
}

impl InternalMethods for Array {
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let PropertyKey::Integer(index) = property_key {
            let elements = agent[self].elements;
            let elements = &agent[elements];
            if let Some(value) = elements.get(index.into_i64() as usize) {
                return Ok(value.map(|value| PropertyDescriptor {
                    value: Some(value),
                    ..Default::default()
                }));
            }
            return Ok(None);
        }
        let length_key = PropertyKey::from(BUILTIN_STRING_MEMORY.length);
        let array_data = agent[self];
        if property_key == length_key {
            Ok(Some(PropertyDescriptor {
                value: Some(array_data.elements.len().into()),
                writable: Some(array_data.elements.len_writable),
                ..Default::default()
            }))
        } else if let Some(object_index) = array_data.object_index {
            Object::Object(object_index).internal_get_own_property(agent, property_key)
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
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            array_set_length(agent, self, property_descriptor)
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if !(0..u32::MAX as i64).contains(&index) {
                return ordinary_define_own_property(
                    agent,
                    self.into_object(),
                    property_key,
                    property_descriptor,
                );
            }
            // Let lengthDesc be OrdinaryGetOwnProperty(A, "length").
            // b. Assert: IsDataDescriptor(lengthDesc) is true.
            // c. Assert: lengthDesc.[[Configurable]] is false.
            // d. Let length be lengthDesc.[[Value]].
            let mut elements = agent[self].elements;
            let length = elements.len();
            // e. Assert: length is a non-negative integral Number.
            // f. Let index be ! ToUint32(P).
            let index = index as u32;
            // g. If index ≥ length and lengthDesc.[[Writable]] is false, return false.
            if index >= length && !agent[self].elements.len_writable {
                return Ok(false);
            }
            // h. Let succeeded be ! OrdinaryDefineOwnProperty(A, P, Desc).
            elements.len = index + 1;
            let elements_data = &mut agent[elements];
            *elements_data.get_mut(index as usize).unwrap() = property_descriptor.value;
            // i. If succeeded is false, return false.
            if false {
                return Ok(false);
            }
            // j. If index ≥ length, then
            if index >= length {
                // i. Set lengthDesc.[[Value]] to index + 1𝔽.
                agent[self].elements.len = index + 1;
                // ii. Set succeeded to ! OrdinaryDefineOwnProperty(A, "length", lengthDesc).
                // iii. Assert: succeeded is true.
            }

            // k. Return true.
            Ok(true)
        } else {
            ordinary_define_own_property(
                agent,
                self.into_object(),
                property_key,
                property_descriptor,
            )
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        let has_own = self.internal_get_own_property(agent, property_key)?;
        if has_own.is_some() {
            return Ok(true);
        }

        // 3. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = self.internal_get_prototype_of(agent)?;

        // 4. If parent is not null, then
        if let Some(parent) = parent {
            // a. Return ? parent.[[HasProperty]](P).
            return parent.internal_has_property(agent, property_key);
        }

        // 5. Return false.
        Ok(false)
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(self.len(agent).into())
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if index < 0 {
                let Some(object_index) = agent[self].object_index else {
                    return Ok(Value::Undefined);
                };
                return object_index.internal_get(agent, property_key, receiver);
            }
            if index >= i64::pow(2, 32) {
                return Ok(Value::Undefined);
            }
            let elements = agent[self].elements;
            if index >= elements.len() as i64 {
                return Ok(Value::Undefined);
            }
            let elements = &agent[elements];
            // Index has been checked to be between 0 <= idx < len; unwrapping should never fail.
            let element = *elements.get(index as usize).unwrap();
            let Some(element) = element else {
                todo!("getters");
            };
            Ok(element)
        } else {
            let Some(object_index) = agent[self].object_index else {
                return Ok(Value::Undefined);
            };
            object_index.internal_get(agent, property_key, receiver)
        }
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(true)
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if index < 0 {
                return agent[self].object_index.map_or(Ok(true), |object_index| {
                    object_index.internal_delete(agent, property_key)
                });
            } else if index >= i64::pow(2, 32) {
                return Ok(true);
            }
            let elements = agent[self].elements;
            if index >= elements.len() as i64 {
                return Ok(true);
            }
            let elements = &mut agent[elements];
            // TODO: Handle unwritable properties
            // Index has been checked to be between 0 <= idx < len; unwrapping should never fail.
            *elements.get_mut(index as usize).unwrap() = None;
            Ok(true)
        } else {
            agent[self].object_index.map_or(Ok(true), |object_index| {
                object_index.internal_delete(agent, property_key)
            })
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        let array_data = &agent[self];
        // TODO: Handle object_index
        let mut keys = Vec::with_capacity(array_data.elements.len() as usize);

        let elements_data = &agent[array_data.elements];

        for (index, value) in elements_data.iter().enumerate() {
            if value.is_some() {
                keys.push(PropertyKey::Integer((index as u32).into()))
            }
        }

        Ok(keys)
    }
}

impl Index<Array> for Agent {
    type Output = ArrayHeapData;

    fn index(&self, index: Array) -> &Self::Output {
        self.heap
            .arrays
            .get(index.0.into_index())
            .expect("Array out of bounds")
            .as_ref()
            .expect("Array slot empty")
    }
}

impl IndexMut<Array> for Agent {
    fn index_mut(&mut self, index: Array) -> &mut Self::Output {
        self.heap
            .arrays
            .get_mut(index.0.into_index())
            .expect("Array out of bounds")
            .as_mut()
            .expect("Array slot empty")
    }
}

impl CreateHeapData<ArrayHeapData, Array> for Heap {
    fn create(&mut self, data: ArrayHeapData) -> Array {
        self.arrays.push(Some(data));
        Array::from(ArrayIndex::last(&self.arrays))
    }
}

impl HeapMarkAndSweep for Array {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.arrays.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        let idx = self.0.into_u32_index();
        self.0 = ArrayIndex::from_u32_index(idx - compactions.arrays.get_shift_for_index(idx));
    }
}
