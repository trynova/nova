// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::Agent,
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData, Value,
            BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
            FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT,
            INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
            UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
            UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
        },
    },
    heap::{indexes::TypedArrayIndex, CreateHeapData, Heap, HeapMarkAndSweep},
};

use self::data::TypedArrayHeapData;

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

impl TypedArray {
    pub(crate) fn get_index(self) -> usize {
        match self {
            TypedArray::Int8Array(index)
            | TypedArray::Uint8Array(index)
            | TypedArray::Uint8ClampedArray(index)
            | TypedArray::Int16Array(index)
            | TypedArray::Uint16Array(index)
            | TypedArray::Int32Array(index)
            | TypedArray::Uint32Array(index)
            | TypedArray::BigInt64Array(index)
            | TypedArray::BigUint64Array(index)
            | TypedArray::Float32Array(index)
            | TypedArray::Float64Array(index) => index.into_index(),
        }
    }
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
        &self.heap.typed_arrays[index]
    }
}

impl IndexMut<TypedArray> for Agent {
    fn index_mut(&mut self, index: TypedArray) -> &mut Self::Output {
        &mut self.heap.typed_arrays[index]
    }
}

impl Index<TypedArray> for Vec<Option<TypedArrayHeapData>> {
    type Output = TypedArrayHeapData;

    fn index(&self, index: TypedArray) -> &Self::Output {
        self.get(index.get_index())
            .expect("TypedArray out of bounds")
            .as_ref()
            .expect("TypedArray slot empty")
    }
}

impl IndexMut<TypedArray> for Vec<Option<TypedArrayHeapData>> {
    fn index_mut(&mut self, index: TypedArray) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("TypedArray out of bounds")
            .as_mut()
            .expect("TypedArray slot empty")
    }
}

impl InternalSlots for TypedArray {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
        debug_assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype,
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
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
}

impl InternalMethods for TypedArray {}

impl CreateHeapData<TypedArrayHeapData, TypedArray> for Heap {
    fn create(&mut self, data: TypedArrayHeapData) -> TypedArray {
        self.typed_arrays.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        TypedArray::Uint8Array(TypedArrayIndex::last(&self.typed_arrays))
    }
}

impl HeapMarkAndSweep for TypedArrayIndex {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.typed_arrays.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.typed_arrays.shift_index(self);
    }
}

impl HeapMarkAndSweep for TypedArray {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        match self {
            TypedArray::Int8Array(data)
            | TypedArray::Uint8Array(data)
            | TypedArray::Uint8ClampedArray(data)
            | TypedArray::Int16Array(data)
            | TypedArray::Uint16Array(data)
            | TypedArray::Int32Array(data)
            | TypedArray::Uint32Array(data)
            | TypedArray::BigInt64Array(data)
            | TypedArray::BigUint64Array(data)
            | TypedArray::Float32Array(data)
            | TypedArray::Float64Array(data) => queues.typed_arrays.push(*data),
        }
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        match self {
            TypedArray::Int8Array(data)
            | TypedArray::Uint8Array(data)
            | TypedArray::Uint8ClampedArray(data)
            | TypedArray::Int16Array(data)
            | TypedArray::Uint16Array(data)
            | TypedArray::Int32Array(data)
            | TypedArray::Uint32Array(data)
            | TypedArray::BigInt64Array(data)
            | TypedArray::BigUint64Array(data)
            | TypedArray::Float32Array(data)
            | TypedArray::Float64Array(data) => compactions.typed_arrays.shift_index(data),
        }
    }
}
