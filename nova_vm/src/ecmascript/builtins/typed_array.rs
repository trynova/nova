use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object,
            OrdinaryObjectInternalSlots, PropertyDescriptor, PropertyKey, Value,
            BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
            FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT,
            INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
            UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
            UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
        },
    },
    heap::{indexes::TypedArrayIndex, Heap, ObjectEntry, ObjectEntryPropertyDescriptor},
};

use self::data::TypedArrayHeapData;

use super::ordinary::ordinary_set_prototype_of_check_loop;

pub mod data;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypedArray {
    Int8Array(TypedArrayIndex) = INT_8_ARRAY_DISCRIMINANT,
    Uint8Array(TypedArrayIndex) = UINT_8_ARRAY_DISCRIMINANT,
    Uint8ClampedArray(TypedArrayIndex) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    Int16Array(TypedArrayIndex) = INT_16_ARRAY_DISCRIMINANT,
    Uint16Array(TypedArrayIndex) = UINT_16_ARRAY_DISCRIMINANT,
    Int32Array(TypedArrayIndex) = INT_32_ARRAY_DISCRIMINANT,
    Uint32Array(TypedArrayIndex) = UINT_32_ARRAY_DISCRIMINANT,
    BigInt64Array(TypedArrayIndex) = BIGINT_64_ARRAY_DISCRIMINANT,
    BigUint64Array(TypedArrayIndex) = BIGUINT_64_ARRAY_DISCRIMINANT,
    Float32Array(TypedArrayIndex) = FLOAT_32_ARRAY_DISCRIMINANT,
    Float64Array(TypedArrayIndex) = FLOAT_64_ARRAY_DISCRIMINANT,
}

impl From<TypedArray> for TypedArrayIndex {
    fn from(val: TypedArray) -> Self {
        match val {
            TypedArray::Int8Array(idx)
            | TypedArray::Uint8Array(idx)
            | TypedArray::Uint8ClampedArray(idx)
            | TypedArray::Int16Array(idx)
            | TypedArray::Uint16Array(idx)
            | TypedArray::Int32Array(idx)
            | TypedArray::Uint32Array(idx)
            | TypedArray::BigInt64Array(idx)
            | TypedArray::BigUint64Array(idx)
            | TypedArray::Float32Array(idx)
            | TypedArray::Float64Array(idx) => idx,
        }
    }
}

impl IntoValue for TypedArray {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for TypedArray {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<TypedArray> for Value {
    fn from(val: TypedArray) -> Self {
        match val {
            TypedArray::Int8Array(idx) => Value::Int8Array(idx),
            TypedArray::Uint8Array(idx) => Value::Uint8Array(idx),
            TypedArray::Uint8ClampedArray(idx) => Value::Uint8ClampedArray(idx),
            TypedArray::Int16Array(idx) => Value::Int16Array(idx),
            TypedArray::Uint16Array(idx) => Value::Uint16Array(idx),
            TypedArray::Int32Array(idx) => Value::Int32Array(idx),
            TypedArray::Uint32Array(idx) => Value::Uint32Array(idx),
            TypedArray::BigInt64Array(idx) => Value::BigInt64Array(idx),
            TypedArray::BigUint64Array(idx) => Value::BigUint64Array(idx),
            TypedArray::Float32Array(idx) => Value::Float32Array(idx),
            TypedArray::Float64Array(idx) => Value::Float64Array(idx),
        }
    }
}

impl From<TypedArray> for Object {
    fn from(val: TypedArray) -> Self {
        match val {
            TypedArray::Int8Array(idx) => Object::Int8Array(idx),
            TypedArray::Uint8Array(idx) => Object::Uint8Array(idx),
            TypedArray::Uint8ClampedArray(idx) => Object::Uint8ClampedArray(idx),
            TypedArray::Int16Array(idx) => Object::Int16Array(idx),
            TypedArray::Uint16Array(idx) => Object::Uint16Array(idx),
            TypedArray::Int32Array(idx) => Object::Int32Array(idx),
            TypedArray::Uint32Array(idx) => Object::Uint32Array(idx),
            TypedArray::BigInt64Array(idx) => Object::BigInt64Array(idx),
            TypedArray::BigUint64Array(idx) => Object::BigUint64Array(idx),
            TypedArray::Float32Array(idx) => Object::Float32Array(idx),
            TypedArray::Float64Array(idx) => Object::Float64Array(idx),
        }
    }
}

impl Index<TypedArray> for Agent {
    type Output = TypedArrayHeapData;

    fn index(&self, index: TypedArray) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<TypedArray> for Agent {
    fn index_mut(&mut self, index: TypedArray) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<TypedArray> for Heap {
    type Output = TypedArrayHeapData;

    fn index(&self, index: TypedArray) -> &Self::Output {
        let index = TypedArrayIndex::from(index).into_index();
        self.typed_arrays
            .get(index)
            .expect("TypedArray out of bounds")
            .as_ref()
            .expect("TypedArray slot empty")
    }
}

impl IndexMut<TypedArray> for Heap {
    fn index_mut(&mut self, index: TypedArray) -> &mut Self::Output {
        let index = TypedArrayIndex::from(index).into_index();
        self.typed_arrays
            .get_mut(index)
            .expect("TypedArray out of bounds")
            .as_mut()
            .expect("TypedArray slot empty")
    }
}

impl OrdinaryObjectInternalSlots for TypedArray {
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
            // Create base object and set inextensible
            todo!()
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
                    .typed_array_prototype()
                    .into_object(),
            )
        }
    }

    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_set_prototype(agent, prototype)
        } else {
            // Create base object and set inextensible
            todo!()
        }
    }
}

impl InternalMethods for TypedArray {
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
            // If we're setting %TypedArray.prototype% then we can still avoid creating the ObjectHeapData.
            let current = agent.current_realm().intrinsics().typed_array_prototype();
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
            let prototype = agent.current_realm().intrinsics().typed_array_prototype();
            let new_entry = ObjectEntry {
                key: property_key,
                value: ObjectEntryPropertyDescriptor::from(property_descriptor),
            };
            let object_index = agent
                .heap
                .create_object_with_prototype(prototype.into_object(), &[new_entry]);
            agent[self].object_index = Some(object_index);
            Ok(true)
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_has_property(agent, property_key)
        } else {
            let parent = self.internal_get_prototype_of(agent)?;
            parent.map_or(Ok(false), |parent| {
                parent.internal_has_property(agent, property_key)
            })
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
            let parent = self.internal_get_prototype_of(agent)?;
            parent.map_or(Ok(Value::Undefined), |parent| {
                parent.internal_get(agent, property_key, receiver)
            })
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
            let prototype = agent.current_realm().intrinsics().typed_array_prototype();
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
