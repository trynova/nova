//! ### 10.4.2 Array Exotic Objects
//!
//! https://tc39.es/ecma262/#sec-array-exotic-objects

pub(crate) mod abstract_operations;
mod data;

use std::ops::Deref;

use super::{
    array_set_length,
    ordinary::{ordinary_define_own_property, ordinary_set},
};
use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::{indexes::ArrayIndex, GetHeapData},
};

impl IntoValue for ArrayIndex {
    fn into_value(self) -> Value {
        Value::Array(self)
    }
}

impl IntoObject for ArrayIndex {
    fn into_object(self) -> Object {
        Object::Array(self)
    }
}

pub use data::{ArrayHeapData, SealableElementsVector};

#[derive(Debug, Clone, Copy)]
pub struct Array(ArrayIndex);

impl Array {
    pub fn len(&self, agent: &Agent) -> u32 {
        agent.heap.get(self.0).elements.len()
    }
}

impl IntoValue for Array {
    fn into_value(self) -> Value {
        Value::Array(self.0)
    }
}

impl IntoObject for Array {
    fn into_object(self) -> Object {
        Object::Array(self.0)
    }
}

impl From<ArrayIndex> for Array {
    fn from(value: ArrayIndex) -> Self {
        Array(value)
    }
}

impl From<Array> for Object {
    fn from(value: Array) -> Self {
        Self::Array(value.0)
    }
}

impl TryFrom<Value> for Array {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Array(data) => Ok(Array(data)),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for Array {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::Array(data) => Ok(Array(data)),
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
        } else if !value {
            // Create array base object and set inextensible
            todo!()
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
                    .array_prototype()
                    .into_object(),
            )
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_set_prototype(agent, prototype)
        } else {
            // Create array base object with custom prototype
            todo!()
        }
    }
}

impl InternalMethods for Array {
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_get_prototype_of(agent)
        } else {
            Ok(Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .array_prototype()
                    .into_object(),
            ))
        }
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_set_prototype_of(agent, prototype)
        } else {
            // 1. Let current be O.[[Prototype]].
            let current = agent.current_realm().intrinsics().array_prototype();
            let object_index = if let Some(v) = prototype {
                if v == current.into_object() {
                    return Ok(true);
                } else {
                    // TODO: Proper handling
                    Some(agent.heap.create_object_with_prototype(v, &[]))
                }
            } else {
                Some(agent.heap.create_null_object(Default::default()))
            };
            agent.heap.get_mut(*self).object_index = object_index;
            Ok(true)
        }
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_is_extensible(agent)
        } else {
            Ok(true)
        }
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_prevent_extensions(agent)
        } else {
            // TODO: Create base array object and call prevent extensions on it.
            Ok(true)
        }
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let PropertyKey::Integer(index) = property_key {
            let elements = agent.heap.get(*self).elements;
            let elements = agent.heap.elements.get(elements.into());
            if let Some(value) = elements.get(index.into_i64() as usize) {
                return Ok(value.map(|value| PropertyDescriptor {
                    value: Some(value),
                    ..Default::default()
                }));
            }
            return Ok(None);
        }
        let length_key = PropertyKey::from(BUILTIN_STRING_MEMORY.length);
        let array_data = agent.heap.get(*self);
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
            let mut elements = agent.heap.get(self.0).elements;
            let length = elements.len();
            // e. Assert: length is a non-negative integral Number.
            // f. Let index be ! ToUint32(P).
            let index = index as u32;
            // g. If index ≥ length and lengthDesc.[[Writable]] is false, return false.
            #[allow(clippy::overly_complex_bool_expr)]
            if index >= length && false {
                // TODO: Handle Array { writable: false }
                return Ok(false);
            }
            // h. Let succeeded be ! OrdinaryDefineOwnProperty(A, P, Desc).
            elements.len = index + 1;
            let elements_data = agent.heap.elements.get_mut(elements.into());
            *elements_data.get_mut(index as usize).unwrap() = property_descriptor.value;
            // i. If succeeded is false, return false.
            if false {
                return Ok(false);
            }
            // j. If index ≥ length, then
            if index >= length {
                // i. Set lengthDesc.[[Value]] to index + 1𝔽.
                agent.heap.get_mut(self.0).elements.len = index + 1;
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
                let Some(object_index) = agent.heap.get(self.0).object_index else {
                    return Ok(Value::Undefined);
                };
                return OrdinaryObject::new(object_index).internal_get(
                    agent,
                    property_key,
                    receiver,
                );
            }
            if index >= i64::pow(2, 32) {
                return Ok(Value::Undefined);
            }
            let elements = agent.heap.get(self.0).elements;
            if index >= elements.len() as i64 {
                return Ok(Value::Undefined);
            }
            let elements = agent.heap.elements.get(elements.into());
            // Index has been checked to be between 0 <= idx < len; unwrapping should never fail.
            let element = *elements.get(index as usize).unwrap();
            let Some(element) = element else {
                todo!("getters");
            };
            Ok(element)
        } else {
            let Some(object_index) = agent.heap.get(self.0).object_index else {
                return Ok(Value::Undefined);
            };
            OrdinaryObject::new(object_index).internal_get(agent, property_key, receiver)
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        ordinary_set(agent, self.into_object(), property_key, value, receiver)
    }

    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
            Ok(true)
        } else if let PropertyKey::Integer(index) = property_key {
            let index = index.into_i64();
            if index < 0 {
                return agent
                    .heap
                    .get(self.0)
                    .object_index
                    .map_or(Ok(true), |object_index| {
                        OrdinaryObject::new(object_index).internal_delete(agent, property_key)
                    });
            } else if index >= i64::pow(2, 32) {
                return Ok(true);
            }
            let elements = agent.heap.get(self.0).elements;
            if index >= elements.len() as i64 {
                return Ok(true);
            }
            let elements = agent.heap.elements.get_mut(elements.into());
            // TODO: Handle unwritable properties
            // Index has been checked to be between 0 <= idx < len; unwrapping should never fail.
            *elements.get_mut(index as usize).unwrap() = None;
            Ok(true)
        } else {
            agent
                .heap
                .get(self.0)
                .object_index
                .map_or(Ok(true), |object_index| {
                    OrdinaryObject::new(object_index).internal_delete(agent, property_key)
                })
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        let array_data = *agent.heap.get(*self);
        // TODO: Handle object_index
        let mut keys = Vec::with_capacity(array_data.elements.len() as usize);

        let elements_data = agent.heap.elements.get(array_data.elements.into());

        for (index, value) in elements_data.iter().enumerate() {
            if value.is_some() {
                keys.push(PropertyKey::Integer((index as u32).into()))
            }
        }

        Ok(keys)
    }
}
