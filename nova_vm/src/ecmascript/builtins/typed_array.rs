// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use data::TypedArrayArrayLength;

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyDescriptor, PropertyKey, Value, BIGINT_64_ARRAY_DISCRIMINANT,
            BIGUINT_64_ARRAY_DISCRIMINANT, FLOAT_32_ARRAY_DISCRIMINANT,
            FLOAT_64_ARRAY_DISCRIMINANT, INT_16_ARRAY_DISCRIMINANT, INT_32_ARRAY_DISCRIMINANT,
            INT_8_ARRAY_DISCRIMINANT, UINT_16_ARRAY_DISCRIMINANT, UINT_32_ARRAY_DISCRIMINANT,
            UINT_8_ARRAY_DISCRIMINANT, UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
        },
    },
    engine::{
        context::{GcScope, NoGcScope},
        rootable::HeapRootData,
        unwrap_try, Scoped, TryResult,
    },
    heap::{
        indexes::{IntoBaseIndex, TypedArrayIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

#[cfg(feature = "proposal-float16array")]
use crate::ecmascript::types::FLOAT_16_ARRAY_DISCRIMINANT;

use self::data::TypedArrayHeapData;

use super::{
    array_buffer::{Ordering, ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
    indexed_collections::typed_array_objects::abstract_operations::{
        is_typed_array_fixed_length, is_typed_array_out_of_bounds, is_valid_integer_index_generic,
        make_typed_array_with_buffer_witness_record, try_typed_array_set_element_generic,
        typed_array_get_element_generic, typed_array_length, typed_array_set_element_generic,
    },
    ordinary::{
        ordinary_define_own_property, ordinary_delete, ordinary_get, ordinary_get_own_property,
        ordinary_has_property_entry, ordinary_prevent_extensions, ordinary_set, ordinary_try_get,
        ordinary_try_has_property_entry, ordinary_try_set,
    },
    ArrayBuffer,
};

pub mod data;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TypedArray<'a> {
    Int8Array(TypedArrayIndex<'a>) = INT_8_ARRAY_DISCRIMINANT,
    Uint8Array(TypedArrayIndex<'a>) = UINT_8_ARRAY_DISCRIMINANT,
    Uint8ClampedArray(TypedArrayIndex<'a>) = UINT_8_CLAMPED_ARRAY_DISCRIMINANT,
    Int16Array(TypedArrayIndex<'a>) = INT_16_ARRAY_DISCRIMINANT,
    Uint16Array(TypedArrayIndex<'a>) = UINT_16_ARRAY_DISCRIMINANT,
    Int32Array(TypedArrayIndex<'a>) = INT_32_ARRAY_DISCRIMINANT,
    Uint32Array(TypedArrayIndex<'a>) = UINT_32_ARRAY_DISCRIMINANT,
    BigInt64Array(TypedArrayIndex<'a>) = BIGINT_64_ARRAY_DISCRIMINANT,
    BigUint64Array(TypedArrayIndex<'a>) = BIGUINT_64_ARRAY_DISCRIMINANT,
    #[cfg(feature = "proposal-float16array")]
    Float16Array(TypedArrayIndex<'a>) = FLOAT_16_ARRAY_DISCRIMINANT,
    Float32Array(TypedArrayIndex<'a>) = FLOAT_32_ARRAY_DISCRIMINANT,
    Float64Array(TypedArrayIndex<'a>) = FLOAT_64_ARRAY_DISCRIMINANT,
}

impl TypedArray<'_> {
    /// Unbind this TypedArray from its current lifetime. This is necessary to use
    /// the TypedArray as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> TypedArray<'static> {
        unsafe { core::mem::transmute::<Self, TypedArray<'static>>(self) }
    }

    // Bind this TypedArray to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your TypedArrays cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let typed_array = typed_array.bind(&gc);
    // ```
    // to make sure that the unbound TypedArray cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> TypedArray<'gc> {
        unsafe { core::mem::transmute::<Self, TypedArray<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, TypedArray<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

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
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(index) => index.into_index(),
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
    pub fn get_viewed_array_buffer<'a>(
        self,
        agent: &Agent,
        _: NoGcScope<'a, '_>,
    ) -> ArrayBuffer<'a> {
        agent[self].viewed_array_buffer
    }
}

impl<'a> From<TypedArrayIndex<'a>> for TypedArray<'a> {
    fn from(value: TypedArrayIndex<'a>) -> Self {
        TypedArray::Uint8Array(value)
    }
}

impl<'a> IntoBaseIndex<'a, TypedArrayHeapData> for TypedArray<'a> {
    fn into_base_index(self) -> TypedArrayIndex<'a> {
        match self {
            TypedArray::Int8Array(i)
            | TypedArray::Uint8Array(i)
            | TypedArray::Uint8ClampedArray(i)
            | TypedArray::Int16Array(i)
            | TypedArray::Uint16Array(i)
            | TypedArray::Int32Array(i)
            | TypedArray::Uint32Array(i)
            | TypedArray::BigInt64Array(i)
            | TypedArray::BigUint64Array(i)
            | TypedArray::Float32Array(i)
            | TypedArray::Float64Array(i) => i,
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(i) => i,
        }
    }
}

impl<'a> From<TypedArray<'a>> for TypedArrayIndex<'a> {
    fn from(val: TypedArray<'a>) -> Self {
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
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(idx) => idx,
        }
    }
}

impl IntoValue for TypedArray<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for TypedArray<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl From<TypedArray<'_>> for Value {
    fn from(val: TypedArray) -> Self {
        match val.unbind() {
            TypedArray::Int8Array(idx) => Value::Int8Array(idx),
            TypedArray::Uint8Array(idx) => Value::Uint8Array(idx),
            TypedArray::Uint8ClampedArray(idx) => Value::Uint8ClampedArray(idx),
            TypedArray::Int16Array(idx) => Value::Int16Array(idx),
            TypedArray::Uint16Array(idx) => Value::Uint16Array(idx),
            TypedArray::Int32Array(idx) => Value::Int32Array(idx),
            TypedArray::Uint32Array(idx) => Value::Uint32Array(idx),
            TypedArray::BigInt64Array(idx) => Value::BigInt64Array(idx),
            TypedArray::BigUint64Array(idx) => Value::BigUint64Array(idx),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(idx) => Value::Float16Array(idx),
            TypedArray::Float32Array(idx) => Value::Float32Array(idx),
            TypedArray::Float64Array(idx) => Value::Float64Array(idx),
        }
    }
}

impl<'a> From<TypedArray<'a>> for Object<'a> {
    fn from(val: TypedArray) -> Self {
        match val.unbind() {
            TypedArray::Int8Array(idx) => Object::Int8Array(idx),
            TypedArray::Uint8Array(idx) => Object::Uint8Array(idx),
            TypedArray::Uint8ClampedArray(idx) => Object::Uint8ClampedArray(idx),
            TypedArray::Int16Array(idx) => Object::Int16Array(idx),
            TypedArray::Uint16Array(idx) => Object::Uint16Array(idx),
            TypedArray::Int32Array(idx) => Object::Int32Array(idx),
            TypedArray::Uint32Array(idx) => Object::Uint32Array(idx),
            TypedArray::BigInt64Array(idx) => Object::BigInt64Array(idx),
            TypedArray::BigUint64Array(idx) => Object::BigUint64Array(idx),
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(idx) => Object::Float16Array(idx),
            TypedArray::Float32Array(idx) => Object::Float32Array(idx),
            TypedArray::Float64Array(idx) => Object::Float64Array(idx),
        }
    }
}

impl TryFrom<Value> for TypedArray<'_> {
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
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array(base_index) => Ok(TypedArray::Float16Array(base_index)),
            Value::Float32Array(base_index) => Ok(TypedArray::Float32Array(base_index)),
            Value::Float64Array(base_index) => Ok(TypedArray::Float64Array(base_index)),
            _ => Err(()),
        }
    }
}

impl Index<TypedArray<'_>> for Agent {
    type Output = TypedArrayHeapData;

    fn index(&self, index: TypedArray) -> &Self::Output {
        &self.heap.typed_arrays[index]
    }
}

impl IndexMut<TypedArray<'_>> for Agent {
    fn index_mut(&mut self, index: TypedArray) -> &mut Self::Output {
        &mut self.heap.typed_arrays[index]
    }
}

impl Index<TypedArray<'_>> for Vec<Option<TypedArrayHeapData>> {
    type Output = TypedArrayHeapData;

    fn index(&self, index: TypedArray) -> &Self::Output {
        self.get(index.get_index())
            .expect("TypedArray out of bounds")
            .as_ref()
            .expect("TypedArray slot empty")
    }
}

impl IndexMut<TypedArray<'_>> for Vec<Option<TypedArrayHeapData>> {
    fn index_mut(&mut self, index: TypedArray) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("TypedArray out of bounds")
            .as_mut()
            .expect("TypedArray slot empty")
    }
}

impl<'a> InternalSlots<'a> for TypedArray<'a> {
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

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
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
                #[cfg(feature = "proposal-float16array")]
                TypedArray::Float16Array(_) => intrinsics.float16_array_prototype(),
                TypedArray::Float32Array(_) => intrinsics.float32_array_prototype(),
                TypedArray::Float64Array(_) => intrinsics.float64_array_prototype(),
            };
            Some(default_proto.into_object())
        }
    }
}

impl<'a> InternalMethods<'a> for TypedArray<'a> {
    /// ### [10.4.5.2 Infallible \[\[GetOwnProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-getownproperty)
    fn try_prevent_extensions(self, agent: &mut Agent, gc: NoGcScope) -> TryResult<bool> {
        // 1. NOTE: The extensibility-related invariants specified in 6.1.7.3
        //    do not allow this method to return true when O can gain (or lose
        //    and then regain) properties, which might occur for properties
        //    with integer index names when its underlying buffer is resized.
        if !is_typed_array_fixed_length(agent, self, gc) {
            // 2. If IsTypedArrayFixedLength(O) is false, return false.
            TryResult::Continue(false)
        } else {
            // 3. Return OrdinaryPreventExtensions(O).
            TryResult::Continue(match self.get_backing_object(agent) {
                Some(backing_object) => ordinary_prevent_extensions(agent, backing_object),
                None => {
                    self.internal_set_extensible(agent, false);
                    true
                }
            })
        }
    }

    /// ### [10.4.5.2 Infallible \[\[GetOwnProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-getownproperty)
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<Option<PropertyDescriptor>> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. Let value be TypedArrayGetElement(O, numericIndex).
            let value = typed_array_get_element_generic(agent, self, numeric_index.into_i64(), gc);
            if let Some(value) = value {
                // iii. Return the PropertyDescriptor {
                //          [[Value]]: value,
                //          [[Writable]]: true,
                //          [[Enumerable]]: true,
                //          [[Configurable]]: true
                //      }.
                TryResult::Continue(Some(PropertyDescriptor {
                    value: Some(value.into_value()),
                    writable: Some(true),
                    enumerable: Some(true),
                    configurable: Some(true),
                    ..Default::default()
                }))
            } else {
                // ii. If value is undefined, return undefined.
                TryResult::Continue(None)
            }
        } else {
            // 2. Return OrdinaryGetOwnProperty(O, P).
            TryResult::Continue(
                self.get_backing_object(agent)
                    .and_then(|object| ordinary_get_own_property(agent, object, property_key)),
            )
        }
    }

    /// ### [10.4.5.3 Infallible \[\[HasProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-hasproperty)
    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, return IsValidIntegerIndex(O, numericIndex).
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            let result = is_valid_integer_index_generic(agent, self, numeric_index, gc);
            TryResult::Continue(result.is_some())
        } else {
            // 2. Return ? OrdinaryHasProperty(O, P).
            ordinary_try_has_property_entry(agent, self, property_key, gc)
        }
    }

    /// ### [10.4.5.3 \[\[HasProperty\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-hasproperty)
    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope,
    ) -> JsResult<bool> {
        if let PropertyKey::Integer(_) = property_key {
            Ok(unwrap_try(self.try_has_property(
                agent,
                property_key,
                gc.into_nogc(),
            )))
        } else {
            // 2. Return ? OrdinaryHasProperty(O, P).
            ordinary_has_property_entry(agent, self, property_key, gc)
        }
    }

    /// ### [10.4.5.4 Infallible \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-typedarray-defineownproperty)
    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return false.
            let numeric_index = numeric_index.into_i64();
            let numeric_index = is_valid_integer_index_generic(agent, self, numeric_index, gc);
            let Some(numeric_index) = numeric_index else {
                return TryResult::Continue(false);
            };
            // ii. If Desc has a [[Configurable]] field and
            //     Desc.[[Configurable]] is false, return false.
            if property_descriptor.configurable == Some(false) {
                return TryResult::Continue(false);
            }
            // iii. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]]
            //      is false, return false.
            if property_descriptor.enumerable == Some(false) {
                return TryResult::Continue(false);
            }
            // iv. If IsAccessorDescriptor(Desc) is true, return false.
            if property_descriptor.is_accessor_descriptor() {
                return TryResult::Continue(false);
            }
            // v. If Desc has a [[Writable]] field and Desc.[[Writable]] is
            //    false, return false.
            if property_descriptor.writable == Some(false) {
                return TryResult::Continue(false);
            }
            // vi. If Desc has a [[Value]] field, perform ?
            //     TypedArraySetElement(O, numericIndex, Desc.[[Value]]).
            if let Some(value) = property_descriptor.value {
                let numeric_index = numeric_index as i64;
                try_typed_array_set_element_generic(agent, self, numeric_index, value, gc)?;
            }
            // vii. Return true.
            TryResult::Continue(true)
        } else {
            // 2. Return ! OrdinaryDefineOwnProperty(O, P, Desc).
            let backing_object = self
                .get_backing_object(agent)
                .unwrap_or_else(|| self.create_backing_object(agent));
            TryResult::Continue(ordinary_define_own_property(
                agent,
                backing_object,
                property_key,
                property_descriptor,
                gc,
            ))
        }
    }

    /// ### [10.4.5.4 \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-typedarray-defineownproperty)
    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope,
    ) -> JsResult<bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return false.
            let numeric_index = numeric_index.into_i64();
            let numeric_index =
                is_valid_integer_index_generic(agent, self, numeric_index, gc.nogc());
            let Some(numeric_index) = numeric_index else {
                return Ok(false);
            };
            // ii. If Desc has a [[Configurable]] field and
            //     Desc.[[Configurable]] is false, return false.
            if property_descriptor.configurable == Some(false) {
                return Ok(false);
            }
            // iii. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]]
            //      is false, return false.
            if property_descriptor.enumerable == Some(false) {
                return Ok(false);
            }
            // iv. If IsAccessorDescriptor(Desc) is true, return false.
            if property_descriptor.is_accessor_descriptor() {
                return Ok(false);
            }
            // v. If Desc has a [[Writable]] field and Desc.[[Writable]] is
            //    false, return false.
            if property_descriptor.writable == Some(false) {
                return Ok(false);
            }
            // vi. If Desc has a [[Value]] field, perform ?
            //     TypedArraySetElement(O, numericIndex, Desc.[[Value]]).
            if let Some(value) = property_descriptor.value {
                let numeric_index = numeric_index as i64;
                typed_array_set_element_generic(agent, self, numeric_index, value, gc)?;
            }
            // vii. Return true.
            Ok(true)
        } else {
            // 2. Return ! OrdinaryDefineOwnProperty(O, P, Desc).
            let backing_object = self
                .get_backing_object(agent)
                .unwrap_or_else(|| self.create_backing_object(agent));
            Ok(ordinary_define_own_property(
                agent,
                backing_object,
                property_key,
                property_descriptor,
                gc.into_nogc(),
            ))
        }
    }

    /// ### [10.4.5.5 Infallible \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-typedarray-get)
    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<Value> {
        // 1. 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            // i. Return TypedArrayGetElement(O, numericIndex).
            let numeric_index = numeric_index.into_i64();
            let result = typed_array_get_element_generic(agent, self, numeric_index, gc);
            TryResult::Continue(result.map_or(Value::Undefined, |v| v.into_value()))
        } else {
            // 2. Return ? OrdinaryGet(O, P, Receiver).
            match self.get_backing_object(agent) {
                Some(backing_object) => {
                    ordinary_try_get(agent, backing_object, property_key, receiver, gc)
                }
                None => {
                    // a. Let parent be ? O.[[GetPrototypeOf]]().
                    let Some(parent) = self.try_get_prototype_of(agent, gc)? else {
                        // b. If parent is null, return undefined.
                        return TryResult::Continue(Value::Undefined);
                    };

                    // c. Return ? parent.[[Get]](P, Receiver).
                    parent.try_get(agent, property_key, receiver, gc)
                }
            }
        }
    }

    /// ### [10.4.5.5 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-typedarray-get)
    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        mut gc: GcScope,
    ) -> JsResult<Value> {
        // 1. 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if property_key.is_array_index() {
            Ok(unwrap_try(self.try_get(
                agent,
                property_key,
                receiver,
                gc.into_nogc(),
            )))
        } else {
            // 2. Return ? OrdinaryGet(O, P, Receiver).
            match self.get_backing_object(agent) {
                Some(backing_object) => {
                    ordinary_get(agent, backing_object, property_key.unbind(), receiver, gc)
                }
                None => {
                    let property_key = property_key.scope(agent, gc.nogc());
                    // a. Let parent be ? O.[[GetPrototypeOf]]().
                    let Some(parent) = self.internal_get_prototype_of(agent, gc.reborrow())? else {
                        // b. If parent is null, return undefined.
                        return Ok(Value::Undefined);
                    };

                    // c. Return ? parent.[[Get]](P, Receiver).
                    parent
                        .unbind()
                        .internal_get(agent, property_key.get(agent), receiver, gc)
                }
            }
        }
    }

    /// ### [10.4.5.6 Infallible \[\[Set\]\] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-typedarray-set)
    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If SameValue(O, Receiver) is true, then
            if self.into_value() == receiver {
                // 1. Perform ? TypedArraySetElement(O, numericIndex, V).
                try_typed_array_set_element_generic(agent, self, numeric_index, value, gc)?;
                // 2. Return true.
                return TryResult::Continue(true);
            } else {
                // ii. If IsValidIntegerIndex(O, numericIndex) is false, return true.
                let result = is_valid_integer_index_generic(agent, self, numeric_index, gc);
                if result.is_none() {
                    return TryResult::Continue(true);
                }
            }
        }
        // 2. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    /// ### [10.4.5.6 \[\[Set\]\] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-typedarray-set)
    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope,
    ) -> JsResult<bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If SameValue(O, Receiver) is true, then
            if self.into_value() == receiver {
                // 1. Perform ? TypedArraySetElement(O, numericIndex, V).
                typed_array_set_element_generic(agent, self, numeric_index, value, gc)?;
                // 2. Return true.
                return Ok(true);
            } else {
                // ii. If IsValidIntegerIndex(O, numericIndex) is false, return true.
                let result = is_valid_integer_index_generic(agent, self, numeric_index, gc.nogc());
                if result.is_none() {
                    return Ok(true);
                }
            }
        }
        // 2. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    /// ### [10.4.5.7 Infallible \[\[Delete\]\] ( P )](https://tc39.es/ecma262/#sec-typedarray-delete)
    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        // 1. If P is a String, then
        // a. Let numericIndex be CanonicalNumericIndexString(P).
        // b. If numericIndex is not undefined, then
        if let PropertyKey::Integer(numeric_index) = property_key {
            let numeric_index = numeric_index.into_i64();
            // i. If IsValidIntegerIndex(O, numericIndex) is false, return true; else return false.
            let numeric_index = is_valid_integer_index_generic(agent, self, numeric_index, gc);
            TryResult::Continue(numeric_index.is_none())
        } else {
            // 2. Return ! OrdinaryDelete(O, P).
            TryResult::Continue(
                self.get_backing_object(agent)
                    .is_none_or(|object| ordinary_delete(agent, object, property_key, gc)),
            )
        }
    }

    /// ### [10.4.5.8 \[\[OwnPropertyKeys\]\] ( )](https://tc39.es/ecma262/#sec-typedarray-ownpropertykeys)
    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        // 1. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, seq-cst).
        let ta_record =
            make_typed_array_with_buffer_witness_record(agent, self, Ordering::SeqCst, gc);
        // 3. If IsTypedArrayOutOfBounds(taRecord) is false, then
        // a. Let length be TypedArrayLength(taRecord).
        let length = match self {
            TypedArray::Int8Array(_)
            | TypedArray::Uint8Array(_)
            | TypedArray::Uint8ClampedArray(_) => {
                if !is_typed_array_out_of_bounds::<u8>(agent, &ta_record, gc) {
                    typed_array_length::<u8>(agent, &ta_record, gc)
                } else {
                    0
                }
            }
            TypedArray::Int16Array(_) | TypedArray::Uint16Array(_) => {
                if !is_typed_array_out_of_bounds::<u16>(agent, &ta_record, gc) {
                    typed_array_length::<u16>(agent, &ta_record, gc)
                } else {
                    0
                }
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => {
                if !is_typed_array_out_of_bounds::<f16>(agent, &ta_record, gc) {
                    typed_array_length::<f16>(agent, &ta_record, gc)
                } else {
                    0
                }
            }
            TypedArray::Int32Array(_)
            | TypedArray::Uint32Array(_)
            | TypedArray::Float32Array(_) => {
                if !is_typed_array_out_of_bounds::<u32>(agent, &ta_record, gc) {
                    typed_array_length::<u32>(agent, &ta_record, gc)
                } else {
                    0
                }
            }
            TypedArray::BigInt64Array(_)
            | TypedArray::BigUint64Array(_)
            | TypedArray::Float64Array(_) => {
                if !is_typed_array_out_of_bounds::<u64>(agent, &ta_record, gc) {
                    typed_array_length::<u64>(agent, &ta_record, gc)
                } else {
                    0
                }
            }
        };
        // 2. Let keys be a new empty List.
        let mut keys = Vec::with_capacity(length);
        // b. For each integer i such that 0 ≤ i < length, in ascending order, do
        // i. Append ! ToString(𝔽(i)) to keys.
        for i in 0..length {
            keys.push(i.try_into().unwrap());
        }
        if let Some(backing_object) = self.get_backing_object(agent) {
            // 4. For each own property key P of O such that P is a String and P is
            //    not an integer index, in ascending chronological order of
            //    property creation, do
            // a. Append P to keys.
            // 5. For each own property key P of O such that P is a Symbol, in
            //    ascending chronological order of property creation, do
            // a. Append P to keys.
            keys.append(&mut unwrap_try(
                backing_object.try_own_property_keys(agent, gc),
            ));
        }
        // 6. Return keys.
        TryResult::Continue(keys)
    }
}

impl TryFrom<HeapRootData> for TypedArray<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::Int8Array(ta) => Ok(Self::Int8Array(ta)),
            HeapRootData::Uint8Array(ta) => Ok(Self::Uint8Array(ta)),
            HeapRootData::Uint8ClampedArray(ta) => Ok(Self::Uint8ClampedArray(ta)),
            HeapRootData::Int16Array(ta) => Ok(Self::Int16Array(ta)),
            HeapRootData::Uint16Array(ta) => Ok(Self::Uint16Array(ta)),
            HeapRootData::Int32Array(ta) => Ok(Self::Int32Array(ta)),
            HeapRootData::Uint32Array(ta) => Ok(Self::Uint32Array(ta)),
            HeapRootData::BigInt64Array(ta) => Ok(Self::BigInt64Array(ta)),
            HeapRootData::BigUint64Array(ta) => Ok(Self::BigUint64Array(ta)),
            // HeapRootData::Float16Array(ta) => Ok(Self::Float16Array(ta)),
            HeapRootData::Float32Array(ta) => Ok(Self::Float32Array(ta)),
            HeapRootData::Float64Array(ta) => Ok(Self::Float64Array(ta)),
            _ => Err(()),
        }
    }
}

impl CreateHeapData<TypedArrayHeapData, TypedArray<'static>> for Heap {
    fn create(&mut self, data: TypedArrayHeapData) -> TypedArray<'static> {
        self.typed_arrays.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        TypedArray::Uint8Array(TypedArrayIndex::last(&self.typed_arrays))
    }
}

impl HeapMarkAndSweep for TypedArrayIndex<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.typed_arrays.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.typed_arrays.shift_index(self);
    }
}

impl HeapMarkAndSweep for TypedArray<'static> {
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
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(data) => queues.typed_arrays.push(*data),
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
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(data) => compactions.typed_arrays.shift_index(data),
        }
    }
}
