// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use data::TypedArrayArrayLength;

use crate::{
    ecmascript::{
        execution::Agent,
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
            BIGINT_64_ARRAY_DISCRIMINANT, BIGUINT_64_ARRAY_DISCRIMINANT,
            FLOAT_32_ARRAY_DISCRIMINANT, FLOAT_64_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT,
            INT_32_ARRAY_DISCRIMINANT, INT_8_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT,
            UINT_32_ARRAY_DISCRIMINANT, UINT_8_ARRAY_DISCRIMINANT,
            UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
        },
    },
    heap::{
        indexes::{IntoBaseIndex, TypedArrayIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::TypedArrayHeapData;

use super::{
    array_buffer::{ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
    ArrayBuffer,
};

pub mod data;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    #[inline]
    pub fn byte_length(self, agent: &Agent) -> Option<usize> {
        let byte_length = agent[self].byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(*agent.heap.typed_array_byte_lengths.get(&self).unwrap())
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }

    #[inline]
    pub fn array_length(self, agent: &Agent) -> Option<usize> {
        let array_length = agent[self].array_length;
        if array_length == TypedArrayArrayLength::heap() {
            Some(*agent.heap.typed_array_array_lengths.get(&self).unwrap())
        } else if array_length == TypedArrayArrayLength::auto() {
            None
        } else {
            Some(array_length.0 as usize)
        }
    }

    #[inline]
    pub fn byte_offset(self, agent: &Agent) -> usize {
        let byte_offset = agent[self].byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *agent.heap.typed_array_byte_offsets.get(&self).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    #[inline]
    pub fn get_viewed_array_buffer(self, agent: &Agent) -> ArrayBuffer {
        agent[self].viewed_array_buffer
    }
}

impl From<TypedArrayIndex> for TypedArray {
    fn from(value: TypedArrayIndex) -> Self {
        TypedArray::Uint8Array(value)
    }
}

impl IntoBaseIndex<'_, TypedArrayHeapData> for TypedArray {
    fn into_base_index(self) -> TypedArrayIndex {
        self.into()
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

impl TryFrom<Value> for TypedArray {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int8Array(base_index) => Ok(TypedArray::Int8Array(base_index)),
            Value::Uint8Array(base_index) => Ok(TypedArray::Uint8Array(base_index)),
            Value::Uint8ClampedArray(base_index) => Ok(TypedArray::Uint8ClampedArray(base_index)),
            Value::Int16Array(base_index) => Ok(TypedArray::Int16Array(base_index)),
            Value::Uint16Array(base_index) => Ok(TypedArray::Uint16Array(base_index)),
            Value::Int32Array(base_index) => Ok(TypedArray::Int32Array(base_index)),
            Value::Uint32Array(base_index) => Ok(TypedArray::Uint32Array(base_index)),
            Value::BigInt64Array(base_index) => Ok(TypedArray::BigInt64Array(base_index)),
            Value::BigUint64Array(base_index) => Ok(TypedArray::BigUint64Array(base_index)),
            Value::Float32Array(base_index) => Ok(TypedArray::Float32Array(base_index)),
            Value::Float64Array(base_index) => Ok(TypedArray::Float64Array(base_index)),
            _ => Err(()),
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
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = agent[self].object_index {
            object_index.internal_prototype(agent)
        } else {
            let intrinsics = agent.current_realm().intrinsics();
            let default_proto = match self {
                TypedArray::Int8Array(_) => intrinsics.int8_array_prototype(),
                TypedArray::Uint8Array(_) => intrinsics.uint8_array_prototype(),
                TypedArray::Uint8ClampedArray(_) => intrinsics.uint8_clamped_array_prototype(),
                TypedArray::Int16Array(_) => intrinsics.int16_array_prototype(),
                TypedArray::Uint16Array(_) => intrinsics.uint16_array_prototype(),
                TypedArray::Int32Array(_) => intrinsics.int32_array_prototype(),
                TypedArray::Uint32Array(_) => intrinsics.uint32_array_prototype(),
                TypedArray::BigInt64Array(_) => intrinsics.big_int64_array_prototype(),
                TypedArray::BigUint64Array(_) => intrinsics.big_int64_array_prototype(),
                TypedArray::Float32Array(_) => intrinsics.float32_array_prototype(),
                TypedArray::Float64Array(_) => intrinsics.float64_array_prototype(),
            };
            Some(default_proto.into_object())
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
