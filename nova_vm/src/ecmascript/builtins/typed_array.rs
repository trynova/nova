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
pub enum TypedArray<'gen> {
    Int8Array(TypedArrayIndex<'gen>) = INT_8_ARRAY_DISCRIMINANT,
    Uint8Array(TypedArrayIndex<'gen>) = UINT_8_ARRAY_DISCRIMINANT,
    Uint8ClampedArray(TypedArrayIndex<'gen>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    Int16Array(TypedArrayIndex<'gen>) = INT_16_ARRAY_DISCRIMINANT,
    Uint16Array(TypedArrayIndex<'gen>) = UINT_16_ARRAY_DISCRIMINANT,
    Int32Array(TypedArrayIndex<'gen>) = INT_32_ARRAY_DISCRIMINANT,
    Uint32Array(TypedArrayIndex<'gen>) = UINT_32_ARRAY_DISCRIMINANT,
    BigInt64Array(TypedArrayIndex<'gen>) = BIGINT_64_ARRAY_DISCRIMINANT,
    BigUint64Array(TypedArrayIndex<'gen>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    Float32Array(TypedArrayIndex<'gen>) = FLOAT_32_ARRAY_DISCRIMINANT,
    Float64Array(TypedArrayIndex<'gen>) = FLOAT_64_ARRAY_DISCRIMINANT,
}

impl TypedArray<'_> {
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

impl<'gen> From<TypedArray<'gen>> for TypedArrayIndex<'gen> {
    fn from(val: TypedArray<'gen>) -> Self {
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

impl<'gen> IntoValue<'gen> for TypedArray<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for TypedArray<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<TypedArray<'gen>> for Value<'gen> {
    fn from(val: TypedArray<'gen>) -> Self {
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

impl<'gen> From<TypedArray<'gen>> for Object<'gen> {
    fn from(val: TypedArray<'gen>) -> Self {
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

impl<'gen> Index<TypedArray<'gen>> for Agent<'gen> {
    type Output = TypedArrayHeapData<'gen>;

    fn index(&self, index: TypedArray<'gen>) -> &Self::Output {
        &self.heap.typed_arrays[index]
    }
}

impl<'gen> IndexMut<TypedArray<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: TypedArray<'gen>) -> &mut Self::Output {
        &mut self.heap.typed_arrays[index]
    }
}

impl<'gen> Index<TypedArray<'gen>> for Vec<Option<TypedArrayHeapData<'gen>>> {
    type Output = TypedArrayHeapData<'gen>;

    fn index(&self, index: TypedArray<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("TypedArray out of bounds")
            .as_ref()
            .expect("TypedArray slot empty")
    }
}

impl<'gen> IndexMut<TypedArray<'gen>> for Vec<Option<TypedArrayHeapData<'gen>>> {
    fn index_mut(&mut self, index: TypedArray<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("TypedArray out of bounds")
            .as_mut()
            .expect("TypedArray slot empty")
    }
}

impl<'gen> InternalSlots<'gen> for TypedArray<'gen> {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
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

    fn internal_prototype(self, agent: &Agent<'gen>) -> Option<Object<'gen>> {
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

impl<'gen> InternalMethods<'gen> for TypedArray<'gen> {}

impl<'gen> CreateHeapData<TypedArrayHeapData<'gen>, TypedArray<'gen>> for Heap<'gen> {
    fn create(&mut self, data: TypedArrayHeapData<'gen>) -> TypedArray<'gen> {
        self.typed_arrays.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        TypedArray::Uint8Array(TypedArrayIndex::last(&self.typed_arrays))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for TypedArrayIndex<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.typed_arrays.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.typed_arrays.shift_index(self);
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for TypedArray<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
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
