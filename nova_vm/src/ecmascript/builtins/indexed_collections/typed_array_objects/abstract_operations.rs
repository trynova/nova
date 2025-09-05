// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::any::TypeId;

use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                construct, get, length_of_array_like, set, species_constructor,
            },
            type_conversion::{IntegerOrInfinity, to_big_int, to_index, to_number, to_object},
        },
        builtins::{
            ArgumentsList, ArrayBuffer, BuiltinFunction,
            array_buffer::{
                Ordering, ViewedArrayBufferByteLength, allocate_array_buffer,
                array_buffer_byte_length, clone_array_buffer, get_value_from_buffer,
                is_detached_buffer, is_fixed_length_array_buffer, set_value_in_buffer,
            },
            indexed_collections::typed_array_objects::typed_array_intrinsic_object::{
                byte_slice_to_viewable, byte_slice_to_viewable_mut,
                require_internal_slot_typed_array, split_typed_array_buffers,
            },
            ordinary::get_prototype_from_constructor,
            typed_array::{
                TypedArray,
                data::{TypedArrayArrayLength, TypedArrayHeapData},
            },
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics,
            agent::{ExceptionType, TryError, TryResult},
        },
        types::{
            BigInt, Function, InternalSlots, IntoFunction, IntoNumeric, IntoObject, IntoValue,
            Number, Numeric, Object, PropertyKey, U8Clamped, Value, Viewable,
        },
    },
    engine::{
        Scoped, ScopedCollection,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::Scopable,
    },
    heap::CreateHeapData,
};

use super::typed_array_intrinsic_object::copy_between_different_type_typed_arrays;

/// Matches a TypedArray and defines a type T in the expression which
/// is the generic type of the viewable.
#[macro_export]
macro_rules! with_typed_array_viewable {
    ($value:expr, $expr:expr) => {
        with_typed_array_viewable!($value, $expr, T)
    };
    ($value:expr, $expr:expr, $as:ident) => {
        match $value {
            TypedArray::Int8Array(_) => {
                type $as = i8;
                $expr
            }
            TypedArray::Uint8Array(_) => {
                type $as = u8;
                $expr
            }
            TypedArray::Uint8ClampedArray(_) => {
                type $as = $crate::ecmascript::types::U8Clamped;
                $expr
            }
            TypedArray::Int16Array(_) => {
                type $as = i16;
                $expr
            }
            TypedArray::Uint16Array(_) => {
                type $as = u16;
                $expr
            }
            TypedArray::Int32Array(_) => {
                type $as = i32;
                $expr
            }
            TypedArray::Uint32Array(_) => {
                type $as = u32;
                $expr
            }
            TypedArray::BigInt64Array(_) => {
                type $as = i64;
                $expr
            }
            TypedArray::BigUint64Array(_) => {
                type $as = u64;
                $expr
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(_) => {
                type $as = f16;
                $expr
            }
            TypedArray::Float32Array(_) => {
                type $as = f32;
                $expr
            }
            TypedArray::Float64Array(_) => {
                type $as = f64;
                $expr
            }
        }
    };
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CachedBufferByteLength(pub usize);

impl CachedBufferByteLength {
    pub const fn value(value: usize) -> Self {
        assert!(
            value != usize::MAX,
            "byte length cannot be usize::MAX as it is reserved for detached buffers"
        );
        Self(value)
    }

    /// A sentinel value of `usize::MAX` means that the buffer is detached.
    pub const fn detached() -> Self {
        Self(usize::MAX)
    }

    pub fn is_detached(self) -> bool {
        self == Self::detached()
    }

    pub fn unwrap(self) -> usize {
        assert_ne!(self.0, usize::MAX, "cannot unwrap a detached buffer");
        self.0
    }
}

impl From<CachedBufferByteLength> for Option<usize> {
    fn from(val: CachedBufferByteLength) -> Self {
        if val.is_detached() { None } else { Some(val.0) }
    }
}

#[derive(Debug)]
pub(crate) struct TypedArrayWithBufferWitnessRecords<'a> {
    pub object: TypedArray<'a>,
    pub cached_buffer_byte_length: CachedBufferByteLength,
}

bindable_handle!(TypedArrayWithBufferWitnessRecords);

/// ### [10.4.5.9 MakeTypedArrayWithBufferWitnessRecord ( obj, order )](https://tc39.es/ecma262/#sec-maketypedarraywithbufferwitnessrecord)
///
/// The abstract operation MakeTypedArrayWithBufferWitnessRecord takes arguments
/// obj (a TypedArray) and order (seq-cst or unordered) and returns a TypedArray
/// With Buffer Witness Record.
pub(crate) fn make_typed_array_with_buffer_witness_record<'a>(
    agent: &Agent,
    obj: TypedArray,
    _order: Ordering,
    gc: NoGcScope<'a, '_>,
) -> TypedArrayWithBufferWitnessRecords<'a> {
    // 1. Let buffer be obj.[[ViewedArrayBuffer]].
    let buffer = obj.get_viewed_array_buffer(agent, gc);

    // 2. If IsDetachedBuffer(buffer) is true, then
    let byte_length = if is_detached_buffer(agent, buffer) {
        // a. Let byteLength be detached.
        CachedBufferByteLength::detached()
    } else {
        // 3. Else,
        // a. Let byteLength be ArrayBufferByteLength(buffer, order).
        CachedBufferByteLength::value(array_buffer_byte_length(agent, buffer))
    };

    // 4. Return the TypedArray With Buffer Witness Record { [[Object]]: obj, [[CachedBufferByteLength]]: byteLength }.
    TypedArrayWithBufferWitnessRecords {
        object: obj.bind(gc),
        cached_buffer_byte_length: byte_length,
    }
}

/// ### [10.4.5.10 TypedArrayCreate ( prototype )](https://tc39.es/ecma262/#sec-typedarraycreate)
///
/// The abstract operation TypedArrayCreate takes argument prototype (an Object)
/// and returns a TypedArray. It is used to specify the creation of new TypedArrays.
pub(crate) fn typed_array_create<'a, T: Viewable>(
    agent: &mut Agent,
    prototype: Option<Object>,
    gc: NoGcScope<'a, '_>,
) -> TypedArray<'a> {
    // 1. Let internalSlotsList be « [[Prototype]], [[Extensible]], [[ViewedArrayBuffer]], [[TypedArrayName]], [[ContentType]], [[ByteLength]], [[ByteOffset]], [[ArrayLength]] ».
    // 2. Let A be MakeBasicObject(internalSlotsList).
    // 3. Set A.[[GetOwnProperty]] as specified in 10.4.5.1.
    // 4. Set A.[[HasProperty]] as specified in 10.4.5.2.
    // 5. Set A.[[DefineOwnProperty]] as specified in 10.4.5.3.
    // 6. Set A.[[Get]] as specified in 10.4.5.4.
    // 7. Set A.[[Set]] as specified in 10.4.5.5.
    // 8. Set A.[[Delete]] as specified in 10.4.5.6.
    // 9. Set A.[[OwnPropertyKeys]] as specified in 10.4.5.7.
    // 10. Set A.[[Prototype]] to prototype.
    // 11. Return A.
    let a = TypedArrayHeapData::new(None);

    let index = agent.heap.create(a);

    let a = match T::PROTO {
        ProtoIntrinsics::Uint8Array => TypedArray::Uint8Array(index),
        ProtoIntrinsics::Uint8ClampedArray => TypedArray::Uint8ClampedArray(index),
        ProtoIntrinsics::Int8Array => TypedArray::Int8Array(index),
        ProtoIntrinsics::Uint16Array => TypedArray::Uint16Array(index),
        ProtoIntrinsics::Int16Array => TypedArray::Int16Array(index),
        ProtoIntrinsics::Uint32Array => TypedArray::Uint32Array(index),
        ProtoIntrinsics::Int32Array => TypedArray::Int32Array(index),
        ProtoIntrinsics::BigUint64Array => TypedArray::BigUint64Array(index),
        ProtoIntrinsics::BigInt64Array => TypedArray::BigInt64Array(index),
        ProtoIntrinsics::Float32Array => TypedArray::Float32Array(index),
        ProtoIntrinsics::Float64Array => TypedArray::Float64Array(index),
        _ => unreachable!(),
    };

    if prototype.is_some() {
        a.internal_set_prototype(agent, prototype);
    }

    a.bind(gc)
}

/// ### [10.4.5.11 TypedArrayByteLength ( taRecord )](https://tc39.es/ecma262/#sec-typedarraybytelength)
///
/// The abstract operation TypedArrayByteLength takes argument taRecord (a
/// TypedArray With Buffer Witness Record) and returns a non-negative integer.
pub(crate) fn typed_array_byte_length<T: Viewable>(
    agent: &mut Agent,
    ta_record: &TypedArrayWithBufferWitnessRecords,
    gc: NoGcScope,
) -> usize {
    // 1. If IsTypedArrayOutOfBounds(taRecord) is true, return 0.
    if is_typed_array_out_of_bounds::<T>(agent, ta_record, gc) {
        return 0;
    }

    // 2. Let length be TypedArrayLength(taRecord).
    let length = typed_array_length::<T>(agent, ta_record, gc);

    // 3. If length = 0, return 0.
    if length == 0 {
        return 0;
    }

    // 4. Let O be taRecord.[[Object]].
    let o = ta_record.object;
    // 5. If O.[[ByteLength]] is not auto, return O.[[ByteLength]].
    if let Some(byte_length) = o.byte_length(agent) {
        return byte_length;
    }

    // 6. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<T>();
    // 7. Return length × elementSize.
    length * element_size
}

/// ### [10.4.5.12 TypedArrayLength ( taRecord )](https://tc39.es/ecma262/#sec-typedarraylength)
///
/// The abstract operation TypedArrayLength takes argument taRecord (a
/// TypedArray With Buffer Witness Record) and returns a non-negative integer.
pub(crate) fn typed_array_length<T: Viewable>(
    agent: &Agent,
    ta_record: &TypedArrayWithBufferWitnessRecords,
    gc: NoGcScope,
) -> usize {
    // 1. Assert: IsTypedArrayOutOfBounds(taRecord) is false.
    assert!(!is_typed_array_out_of_bounds::<T>(agent, ta_record, gc));

    // 2. Let O be taRecord.[[Object]].
    let o = ta_record.object;

    // 3. If O.[[ArrayLength]] is not auto, return O.[[ArrayLength]].
    if let Some(array_length) = o.array_length(agent) {
        return array_length;
    }

    // 4. Assert: IsFixedLengthArrayBuffer(O.[[ViewedArrayBuffer]]) is false.
    assert!(!is_fixed_length_array_buffer(
        agent,
        o.get_viewed_array_buffer(agent, gc)
    ));

    // 5. Let byteOffset be O.[[ByteOffset]].
    let byte_offset = o.byte_offset(agent);

    // 6. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<T>();

    // 7. Let byteLength be taRecord.[[CachedBufferByteLength]].
    // 8. Assert: byteLength is not detached.
    let byte_length = ta_record.cached_buffer_byte_length.0;

    // 9. Return floor((byteLength - byteOffset) / elementSize).
    (byte_length - byte_offset) / element_size
}

/// ### [10.4.5.13 IsTypedArrayOutOfBounds ( taRecord )](https://tc39.es/ecma262/#sec-istypedarrayoutofbounds)
///
/// The abstract operation IsTypedArrayOutOfBounds takes argument taRecord (a
/// TypedArray With Buffer Witness Record) and returns a Boolean. It checks if
/// any of the object's numeric properties reference a value at an index not
/// contained within the underlying buffer's bounds.
pub(crate) fn is_typed_array_out_of_bounds<T: Viewable>(
    agent: &Agent,
    ta_record: &TypedArrayWithBufferWitnessRecords,
    gc: NoGcScope,
) -> bool {
    // 1. Let O be taRecord.[[Object]].
    let o = ta_record.object;

    // 2. Let bufferByteLength be taRecord.[[CachedBufferByteLength]].
    let buffer_byte_length = ta_record.cached_buffer_byte_length;

    // 3. Assert: IsDetachedBuffer(O.[[ViewedArrayBuffer]]) is true if and only if bufferByteLength is detached.
    assert_eq!(
        is_detached_buffer(agent, o.get_viewed_array_buffer(agent, gc)),
        buffer_byte_length.is_detached()
    );

    // 4. If bufferByteLength is detached, return true.
    let Some(buffer_byte_length) = buffer_byte_length.into() else {
        return true;
    };

    // 5. Let byteOffsetStart be O.[[ByteOffset]].
    let byte_offset_start = o.byte_offset(agent);

    // 6. If O.[[ArrayLength]] is auto, then
    let byte_offset_end = if let Some(array_length) = o.array_length(agent) {
        // 7. Else,
        // a. Let elementSize be TypedArrayElementSize(O).
        let element_size = size_of::<T>();
        // b. Let byteOffsetEnd be byteOffsetStart + O.[[ArrayLength]] × elementSize.
        byte_offset_start + array_length * element_size
    } else {
        // a. Let byteOffsetEnd be bufferByteLength.
        buffer_byte_length
    };

    // 8. If byteOffsetStart > bufferByteLength or byteOffsetEnd > bufferByteLength, return true.
    if byte_offset_start > buffer_byte_length || byte_offset_end > buffer_byte_length {
        return true;
    }

    // 9. NOTE: 0-length TypedArrays are not considered out-of-bounds.
    // 10. Return false.
    false
}

/// ### [10.4.5.15 IsTypedArrayFixedLength ( O )](https://tc39.es/ecma262/#sec-istypedarrayfixedlength)
///
/// The abstract operation IsTypedArrayFixedLength takes argument O (a
/// TypedArray) and returns a Boolean.
pub(crate) fn is_typed_array_fixed_length(agent: &Agent, o: TypedArray, gc: NoGcScope) -> bool {
    // 1. If O.[[ArrayLength]] is auto, return false.
    if o.array_length(agent).is_none() {
        false
    } else {
        // 2. Let buffer be O.[[ViewedArrayBuffer]].
        let buffer = o.get_viewed_array_buffer(agent, gc);
        // 3. If IsFixedLengthArrayBuffer(buffer) is false and IsSharedArrayBuffer(buffer) is false, return false.
        if !is_fixed_length_array_buffer(agent, buffer)
        // && !is_shared_array_buffer(agent, buffer)
        {
            false
        } else {
            // 4. Return true.
            true
        }
    }
}

/// ### [10.4.5.16 Generic IsValidIntegerIndex ( O, index )](https://tc39.es/ecma262/#sec-isvalidintegerindex)
///
/// The abstract operation IsValidIntegerIndex takes arguments O (a TypedArray)
/// and index (a Number) and returns a Boolean.
pub(crate) fn is_valid_integer_index_generic(
    agent: &Agent,
    o: TypedArray,
    index: i64,
    gc: NoGcScope,
) -> Option<usize> {
    with_typed_array_viewable!(o, is_valid_integer_index::<T>(agent, o, index, gc))
}

/// ### [10.4.5.16 IsValidIntegerIndex ( O, index )](https://tc39.es/ecma262/#sec-isvalidintegerindex)
///
/// The abstract operation IsValidIntegerIndex takes arguments O (a TypedArray)
/// and index (a Number) and returns a Boolean.
pub(crate) fn is_valid_integer_index<O: Viewable>(
    agent: &Agent,
    o: TypedArray,
    index: i64,
    gc: NoGcScope,
) -> Option<usize> {
    // 1. If IsDetachedBuffer(O.[[ViewedArrayBuffer]]) is true, return false.
    if is_detached_buffer(agent, o.get_viewed_array_buffer(agent, gc)) {
        return None;
    }
    // 2. If index is not an integral Number, return false.
    // 3. If index is -0𝔽 or index < -0𝔽, return false.
    if index < 0 {
        return None;
    }
    let index = index as usize;
    // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, unordered).
    let ta_record = make_typed_array_with_buffer_witness_record(agent, o, Ordering::Unordered, gc);
    // 5. NOTE: Bounds checking is not a synchronizing operation when O's
    //    backing buffer is a growable SharedArrayBuffer.
    // 6. If IsTypedArrayOutOfBounds(taRecord) is true, return false.
    if is_typed_array_out_of_bounds::<O>(agent, &ta_record, gc) {
        return None;
    }
    // 7. Let length be TypedArrayLength(taRecord).
    let length = typed_array_length::<O>(agent, &ta_record, gc);
    // 8. If ℝ(index) ≥ length, return false.
    if index >= length {
        None
    } else {
        // 9. Return true.
        Some(index)
    }
}

/// ### [10.4.5.17 TypedArrayGetElement ( O, index )](https://tc39.es/ecma262/#sec-typedarraygetelement)
///
/// The abstract operation TypedArrayGetElement takes arguments O (a
/// TypedArray) and index (a Number) and returns a Number, a BigInt,
/// or undefined.
pub(crate) fn typed_array_get_element_generic<'a>(
    agent: &mut Agent,
    o: TypedArray,
    index: i64,
    gc: NoGcScope<'a, '_>,
) -> Option<Numeric<'a>> {
    with_typed_array_viewable!(o, typed_array_get_element::<T>(agent, o, index, gc))
}

/// ### [10.4.5.17 TypedArrayGetElement ( O, index )](https://tc39.es/ecma262/#sec-typedarraygetelement)
///
/// The abstract operation TypedArrayGetElement takes arguments O (a
/// TypedArray) and index (a Number) and returns a Number, a BigInt,
/// or undefined.
pub(crate) fn typed_array_get_element<'a, O: Viewable>(
    agent: &mut Agent,
    o: TypedArray,
    index: i64,
    gc: NoGcScope<'a, '_>,
) -> Option<Numeric<'a>> {
    // 1. If IsValidIntegerIndex(O, index) is false, return undefined.
    let index = is_valid_integer_index::<O>(agent, o, index, gc)?;
    // 2. Let offset be O.[[ByteOffset]].
    let offset = o.byte_offset(agent);
    // 3. Let elementSize be TypedArrayElementSize(O).
    let element_size = core::mem::size_of::<O>();
    // 4. Let byteIndexInBuffer be (ℝ(index) × elementSize) + offset.
    let byte_index_in_buffer = (index * element_size) + offset;
    // 5. Let elementType be TypedArrayElementType(O).
    // 6. Return GetValueFromBuffer(O.[[ViewedArrayBuffer]], byteIndexInBuffer, elementType, true, unordered).
    Some(get_value_from_buffer::<O>(
        agent,
        o.get_viewed_array_buffer(agent, gc),
        byte_index_in_buffer,
        true,
        Ordering::Unordered,
        None,
        gc,
    ))
}

/// ### [10.4.5.18 Generic TypedArraySetElement ( O, index, value )](https://tc39.es/ecma262/#sec-typedarraysetelement)
///
/// The abstract operation TypedArraySetElement takes arguments O (a
/// TypedArray), index (a Number), and value (an ECMAScript language value) and
/// returns either a normal completion containing unused or a throw completion.
///
/// > Note
/// >
/// > This operation always appears to succeed, but it has no effect when
/// > attempting to write past the end of a TypedArray or to a TypedArray which
/// > is backed by a detached ArrayBuffer.
pub(crate) fn typed_array_set_element_generic<'a>(
    agent: &mut Agent,
    o: TypedArray,
    index: i64,
    value: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    with_typed_array_viewable!(o, typed_array_set_element::<T>(agent, o, index, value, gc))
}

/// ### [10.4.5.18 TypedArraySetElement ( O, index, value )](https://tc39.es/ecma262/#sec-typedarraysetelement)
///
/// The abstract operation TypedArraySetElement takes arguments O (a
/// TypedArray), index (a Number), and value (an ECMAScript language value) and
/// returns either a normal completion containing unused or a throw completion.
///
/// > Note
/// >
/// > This operation always appears to succeed, but it has no effect when
/// > attempting to write past the end of a TypedArray or to a TypedArray which
/// > is backed by a detached ArrayBuffer.
pub(crate) fn typed_array_set_element<'a, O: Viewable>(
    agent: &mut Agent,
    o: TypedArray,
    index: i64,
    value: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let mut o = o.bind(gc.nogc());
    let value = value.bind(gc.nogc());
    // 1. If O.[[ContentType]] is bigint, let numValue be ? ToBigInt(value).
    let num_value = if O::IS_BIGINT {
        if let Ok(v) = BigInt::try_from(value) {
            v.into_numeric()
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let v = to_big_int(agent, value.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
                .into_numeric();
            o = scoped_o.get(agent).bind(gc.nogc());
            v
        }
    } else {
        // 2. Otherwise, let numValue be ? ToNumber(value).
        if let Ok(v) = Number::try_from(value) {
            v.into_numeric()
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let v = to_number(agent, value.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
                .into_numeric();
            o = scoped_o.get(agent).bind(gc.nogc());
            v
        }
    };
    let o = o.unbind();
    let num_value = num_value.unbind();
    let gc = gc.into_nogc();
    let o = o.bind(gc);
    let num_value = num_value.bind(gc);
    typed_array_set_element_internal::<O>(agent, o, index, num_value, gc);
    // 4. Return unused.
    Ok(())
}

/// ### [10.4.5.18 Infallible generic TypedArraySetElement ( O, index, value )](https://tc39.es/ecma262/#sec-typedarraysetelement)
///
/// The abstract operation TypedArraySetElement takes arguments O (a
/// TypedArray), index (a Number), and value (an ECMAScript language value) and
/// returns either a normal completion containing unused or a throw completion.
///
/// > Note
/// >
/// > This operation always appears to succeed, but it has no effect when
/// > attempting to write past the end of a TypedArray or to a TypedArray which
/// > is backed by a detached ArrayBuffer.
pub(crate) fn try_typed_array_set_element_generic<'gc>(
    agent: &mut Agent,
    o: TypedArray,
    index: i64,
    value: Value,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, ()> {
    with_typed_array_viewable!(
        o,
        try_typed_array_set_element::<T>(agent, o, index, value, gc)
    )
}

/// ### [10.4.5.18 Infallible TypedArraySetElement ( O, index, value )](https://tc39.es/ecma262/#sec-typedarraysetelement)
///
/// The abstract operation TypedArraySetElement takes arguments O (a
/// TypedArray), index (a Number), and value (an ECMAScript language value) and
/// returns either a normal completion containing unused or a throw completion.
///
/// > Note
/// >
/// > This operation always appears to succeed, but it has no effect when
/// > attempting to write past the end of a TypedArray or to a TypedArray which
/// > is backed by a detached ArrayBuffer.
pub(crate) fn try_typed_array_set_element<'gc, O: Viewable>(
    agent: &mut Agent,
    o: TypedArray,
    index: i64,
    value: Value,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, ()> {
    // 1. If O.[[ContentType]] is bigint, let numValue be ? ToBigInt(value).
    let num_value = if O::IS_BIGINT {
        if let Ok(v) = BigInt::try_from(value) {
            v.into_numeric()
        } else {
            return TryError::GcError.into();
        }
    } else {
        // 2. Otherwise, let numValue be ? ToNumber(value).
        if let Ok(v) = Number::try_from(value) {
            v.into_numeric()
        } else {
            return TryError::GcError.into();
        }
    };
    typed_array_set_element_internal::<O>(agent, o, index, num_value, gc);
    // 4. Return unused.
    TryResult::Continue(())
}

fn typed_array_set_element_internal<O: Viewable>(
    agent: &mut Agent,
    o: TypedArray,
    index: i64,
    num_value: Numeric,
    gc: NoGcScope,
) {
    // 3. If IsValidIntegerIndex(O, index) is true, then
    if let Some(index) = is_valid_integer_index::<O>(agent, o, index, gc) {
        // a. Let offset be O.[[ByteOffset]].
        let offset = o.byte_offset(agent);
        // b. Let elementSize be TypedArrayElementSize(O).
        let element_size = core::mem::size_of::<O>();
        // c. Let byteIndexInBuffer be (ℝ(index) × elementSize) + offset.
        let byte_index_in_buffer = index * element_size + offset;
        // d. Let elementType be TypedArrayElementType(O).
        // e. Perform SetValueInBuffer(O.[[ViewedArrayBuffer]], byteIndexInBuffer, elementType, numValue, true, unordered).
        set_value_in_buffer::<O>(
            agent,
            o.get_viewed_array_buffer(agent, gc),
            byte_index_in_buffer,
            num_value,
            true,
            Ordering::Unordered,
            None,
        );
    }
}

/// ### [23.2.4.4 ValidateTypedArray ( O, order )](https://tc39.es/ecma262/#sec-validatetypedarray)
///
/// The abstract operation ValidateTypedArray takes arguments O (an ECMAScript
/// language value) and order (seq-cst or unordered) and returns either a normal
/// completion containing a TypedArray With Buffer Witness Record or a throw
/// completion.
pub(crate) fn validate_typed_array<'a>(
    agent: &mut Agent,
    o: Value,
    order: Ordering,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, TypedArrayWithBufferWitnessRecords<'a>> {
    // 1. Perform ? RequireInternalSlot(O, [[TypedArrayName]]).
    let o = require_internal_slot_typed_array(agent, o, gc)?;
    // 2. Assert: O has a [[ViewedArrayBuffer]] internal slot.
    // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
    let ta_record = make_typed_array_with_buffer_witness_record(agent, o, order, gc);
    // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
    if with_typed_array_viewable!(o, is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc)) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc,
        ));
    }

    // 5. Return taRecord.
    Ok(ta_record)
}

/// ### [23.2.5.1.1 AllocateTypedArray ( constructorName, newTarget, defaultProto \[ , length \] )](https://tc39.es/ecma262/#sec-allocatetypedarray)
///
/// The abstract operation AllocateTypedArray takes arguments constructorName
/// (a String which is the name of a TypedArray constructor in Table 69),
/// newTarget (a constructor), and defaultProto (a String) and optional argument
/// length (a non-negative integer) and returns either a normal completion
/// containing a TypedArray or a throw completion. It is used to validate and
/// create an instance of a TypedArray constructor. If the length argument is
/// passed, an ArrayBuffer of that length is also allocated and associated with
/// the new TypedArray instance. AllocateTypedArray provides common semantics
/// that is used by TypedArray.
pub(crate) fn allocate_typed_array<'a, T: Viewable>(
    agent: &mut Agent,
    new_target: Function,
    default_proto: ProtoIntrinsics,
    length: Option<usize>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let new_target = new_target.bind(gc.nogc());
    // 1. Let proto be ? GetPrototypeFromConstructor(newTarget, defaultProto).
    let proto =
        get_prototype_from_constructor(agent, new_target.unbind(), default_proto, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

    // 2. Let obj be TypedArrayCreate(proto).
    let obj = typed_array_create::<T>(agent, proto, gc.nogc());

    // NOTE: Steps 3-7 are skipped, it's the defaults for TypedArrayHeapData.
    // 3. Assert: obj.[[ViewedArrayBuffer]] is undefined.
    // 4. Set obj.[[TypedArrayName]] to constructorName.
    // 5. If constructorName is either "BigInt64Array" or "BigUint64Array", set obj.[[ContentType]] to bigint.
    // 6. Otherwise, set obj.[[ContentType]] to number.
    // 7. If length is not present, then
    // a. Set obj.[[ByteLength]] to 0.
    // b. Set obj.[[ByteOffset]] to 0.
    // c. Set obj.[[ArrayLength]] to 0.

    if let Some(length) = length {
        // 8. Else,
        // a. Perform ? AllocateTypedArrayBuffer(obj, length).
        allocate_typed_array_buffer::<T>(agent, obj, length, gc.nogc()).unbind()?;
    }

    // 9. Return obj.
    Ok(obj.unbind().bind(gc.into_nogc()))
}

/// ### [23.2.5.1.2 InitializeTypedArrayFromTypedArray ( O, srcArray )](https://tc39.es/ecma262/#sec-initializetypedarrayfromtypedarray)
///
/// The abstract operation InitializeTypedArrayFromTypedArray takes arguments O
/// (a TypedArray) and srcArray (a TypedArray) and returns either a normal
/// completion containing unused or a throw completion.
pub(crate) fn initialize_typed_array_from_typed_array<'a, O: Viewable, Src: Viewable>(
    agent: &mut Agent,
    o: TypedArray,
    src_array: TypedArray,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let src_heap_data = &agent[src_array];

    // 1. Let srcData be srcArray.[[ViewedArrayBuffer]].
    let src_data = src_heap_data.viewed_array_buffer;

    // 2. Let elementType be TypedArrayElementType(O).
    // 3. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<O>();

    // 4. Let srcType be TypedArrayElementType(srcArray).
    // 5. Let srcElementSize be TypedArrayElementSize(srcArray).
    let src_element_size = size_of::<Src>();

    // 6. Let srcByteOffset be srcArray.[[ByteOffset]].
    let src_byte_offset = src_array.byte_offset(agent);

    // 7. Let srcRecord be MakeTypedArrayWithBufferWitnessRecord(srcArray, seq-cst).
    let src_record =
        make_typed_array_with_buffer_witness_record(agent, src_array, Ordering::SeqCst, gc);

    // 8. If IsTypedArrayOutOfBounds(srcRecord) is true, throw a TypeError exception.
    if is_typed_array_out_of_bounds::<Src>(agent, &src_record, gc) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc,
        ));
    }

    // 9. Let elementLength be TypedArrayLength(srcRecord).
    let element_length = typed_array_length::<Src>(agent, &src_record, gc);

    // 10. Let byteLength be elementSize × elementLength.
    let byte_length = element_size * element_length;

    // 11. If elementType is srcType, then
    let data = if O::PROTO == Src::PROTO {
        // a. Let data be ? CloneArrayBuffer(srcData, srcByteOffset, byteLength).
        clone_array_buffer(agent, src_data, src_byte_offset, byte_length, gc)?
    } else {
        // 12. Else,
        // a. Let data be ? AllocateArrayBuffer(%ArrayBuffer%, byteLength).
        let array_buffer_constructor = agent
            .current_realm_record()
            .intrinsics()
            .array_buffer()
            .bind(gc);
        let data = allocate_array_buffer(
            agent,
            array_buffer_constructor.into_function(),
            byte_length as u64,
            None,
            gc,
        )?;

        // b. If srcArray.[[ContentType]] is not O.[[ContentType]], throw a TypeError exception.
        if O::IS_BIGINT != Src::IS_BIGINT {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "TypedArray content type mismatch",
                gc,
            ));
        }

        // c. Let srcByteIndex be srcByteOffset.
        let mut src_byte_index = src_byte_offset;
        // d. Let targetByteIndex be 0.
        let mut target_byte_index = 0;
        // e. Let count be elementLength.
        let mut count = element_length;
        // f. Repeat, while count > 0,
        while count != 0 {
            // i. Let value be GetValueFromBuffer(srcData, srcByteIndex, srcType, true, unordered).
            let value = get_value_from_buffer::<Src>(
                agent,
                src_data,
                src_byte_index,
                true,
                Ordering::Unordered,
                None,
                gc,
            );

            // ii. Perform SetValueInBuffer(data, targetByteIndex, elementType, value, true, unordered).
            set_value_in_buffer::<O>(
                agent,
                data,
                target_byte_index,
                value,
                true,
                Ordering::Unordered,
                None,
            );

            // iii. Set srcByteIndex to srcByteIndex + srcElementSize.
            src_byte_index += src_element_size;
            // iv. Set targetByteIndex to targetByteIndex + elementSize.
            target_byte_index += element_size;
            // v. Set count to count - 1.
            count -= 1;
        }
        data
    };

    let o_heap_data = &mut agent[o];

    let heap_byte_length = byte_length.into();
    let heap_array_length = element_length.into();

    // 13. Set O.[[ViewedArrayBuffer]] to data.
    o_heap_data.viewed_array_buffer = data.unbind();
    // 14. Set O.[[ByteLength]] to byteLength.
    o_heap_data.byte_length = heap_byte_length;
    // 15. Set O.[[ByteOffset]] to 0.
    o_heap_data.byte_offset = 0.into();
    // 16. Set O.[[ArrayLength]] to elementLength.
    o_heap_data.array_length = heap_array_length;

    if heap_byte_length.is_overflowing() {
        o.set_overflowing_byte_length(agent, byte_length);
        // Note: if byte length doesn't overflow then array length cannot
        // overflow either.
        if heap_array_length.is_overflowing() {
            o.set_overflowing_array_length(agent, element_length);
        }
    }

    // 17. Return unused.
    Ok(())
}

/// ### [23.2.5.1.3 InitializeTypedArrayFromArrayBuffer ( O, buffer, byteOffset, length )](https://tc39.es/ecma262/#sec-initializetypedarrayfromarraybuffer)
///
/// The abstract operation InitializeTypedArrayFromArrayBuffer takes arguments
/// O (a TypedArray), buffer (an ArrayBuffer or a SharedArrayBuffer),
/// byteOffset (an ECMAScript language value), and length (an ECMAScript
/// language value) and returns either a normal completion containing unused or
/// a throw completion.
pub(crate) fn initialize_typed_array_from_array_buffer<'a, T: Viewable>(
    agent: &mut Agent,
    scoped_o: Scoped<TypedArray>,
    scoped_buffer: Scoped<ArrayBuffer>,
    byte_offset: Option<Scoped<Value>>,
    length: Option<Scoped<Value>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<T>();

    // 2. Let offset be ? ToIndex(byteOffset).
    let offset = if let Some(byte_offset) = byte_offset {
        to_index(agent, byte_offset.get(agent), gc.reborrow()).unbind()? as usize
    } else {
        0
    };

    // 3. If offset modulo elementSize ≠ 0, throw a RangeError exception.
    if !offset.is_multiple_of(element_size) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "offset is not a multiple of the element size",
            gc.into_nogc(),
        ));
    }

    let buffer = scoped_buffer.get(agent).bind(gc.nogc());
    // 4. Let bufferIsFixedLength be IsFixedLengthArrayBuffer(buffer).
    let buffer_is_fixed_length = is_fixed_length_array_buffer(agent, buffer);

    // 5. If length is not undefined, then
    // a. Let newLength be ? ToIndex(length).
    let new_length = if let Some(length) = length {
        let length = length.get(agent).bind(gc.nogc());
        if length.is_undefined() {
            None
        } else {
            Some(to_index(agent, length.unbind(), gc.reborrow()).unbind()? as usize)
        }
    } else {
        None
    };

    let buffer = scoped_buffer.get(agent).bind(gc.nogc());
    // 6. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
    if is_detached_buffer(agent, buffer) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "attempting to access detached ArrayBuffer",
            gc.into_nogc(),
        ));
    }

    // 7. Let bufferByteLength be ArrayBufferByteLength(buffer, seq-cst).
    let buffer_byte_length = array_buffer_byte_length(agent, buffer);

    let o = scoped_o.get(agent).bind(gc.nogc());
    let o_heap_data = &mut agent[o];

    // 8. If length is undefined and bufferIsFixedLength is false, then
    if new_length.is_none() && !buffer_is_fixed_length {
        // a. If offset > bufferByteLength, throw a RangeError exception.
        if offset > buffer_byte_length {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "offset is outside the bounds of the buffer",
                gc.into_nogc(),
            ));
        }

        let heap_byte_offset = offset.into();

        // b. Set O.[[ByteLength]] to auto.
        o_heap_data.byte_length = ViewedArrayBufferByteLength::auto();
        // c. Set O.[[ArrayLength]] to auto.
        o_heap_data.array_length = TypedArrayArrayLength::auto();
        // 10. Set O.[[ViewedArrayBuffer]] to buffer.
        o_heap_data.viewed_array_buffer = buffer.unbind();
        // 11. Set O.[[ByteOffset]] to offset.
        o_heap_data.byte_offset = heap_byte_offset;

        if heap_byte_offset.is_overflowing() {
            o.set_overflowing_byte_offset(agent, offset);
        }
    } else {
        // 9. Else,
        let new_byte_length = if let Some(new_length) = new_length {
            // b. Else,
            // i. Let newByteLength be newLength × elementSize.
            let new_byte_length = new_length * element_size;
            // ii. If offset + newByteLength > bufferByteLength, throw a RangeError exception.
            if offset + new_byte_length > buffer_byte_length {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "offset is outside the bounds of the buffer",
                    gc.into_nogc(),
                ));
            }

            new_byte_length
        } else
        // a. If length is undefined, then
        // i. If bufferByteLength modulo elementSize ≠ 0, throw a RangeError exception.
        if !buffer_byte_length.is_multiple_of(element_size) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "buffer length is not a multiple of the element size",
                gc.into_nogc(),
            ));
        } else
        // ii. Let newByteLength be bufferByteLength - offset.
        if let Some(new_byte_length) = buffer_byte_length.checked_sub(offset) {
            new_byte_length
        } else {
            // iii. If newByteLength < 0, throw a RangeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "new byte length is negative",
                gc.into_nogc(),
            ));
        };

        let heap_byte_length = new_byte_length.into();
        let length = new_byte_length / element_size;
        let heap_array_length = length.into();
        let heap_byte_offset = offset.into();

        // c. Set O.[[ByteLength]] to newByteLength.
        o_heap_data.byte_length = heap_byte_length;
        // d. Set O.[[ArrayLength]] to newByteLength / elementSize.
        o_heap_data.array_length = heap_array_length;
        // 10. Set O.[[ViewedArrayBuffer]] to buffer.
        o_heap_data.viewed_array_buffer = buffer.unbind();
        // 11. Set O.[[ByteOffset]] to offset.
        o_heap_data.byte_offset = heap_byte_offset;

        if heap_byte_length.is_overflowing() {
            o.set_overflowing_byte_length(agent, new_byte_length);
            // Note: if byte length doesn't overflow then array length cannot
            // overflow either.
            if heap_array_length.is_overflowing() {
                o.set_overflowing_array_length(agent, length);
            }
        }
        if heap_byte_offset.is_overflowing() {
            o.set_overflowing_byte_offset(agent, offset);
        }
    }

    // 12. Return unused.
    Ok(())
}

/// ### [23.2.5.1.4 InitializeTypedArrayFromList ( O, values )](https://tc39.es/ecma262/#sec-initializetypedarrayfromlist)
///
/// The abstract operation InitializeTypedArrayFromList takes arguments O (a
/// TypedArray) and values (a List of ECMAScript language values) and returns
/// either a normal completion containing unused or a throw completion.
pub(crate) fn initialize_typed_array_from_list<'a, T: Viewable>(
    agent: &mut Agent,
    scoped_o: Scoped<TypedArray>,
    values: ScopedCollection<Vec<Value>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let mut o = scoped_o.get(agent).bind(gc.nogc());
    // 1. Let len be the number of elements in values.
    // 2. Perform ? AllocateTypedArrayBuffer(O, len).
    allocate_typed_array_buffer::<T>(agent, o, values.len(agent), gc.nogc()).unbind()?;

    // 3. Let k be 0.
    // 4. Repeat, while k < len,
    // b. Let kValue be the first element of values.
    // c. Remove the first element from values.
    // e. Set k to k + 1.
    for (k, k_value) in values.iter(agent).enumerate() {
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk = PropertyKey::from(SmallInteger::try_from(k as i64).unwrap());
        let k_value = k_value.get(gc.nogc());
        // d. Perform ? Set(O, Pk, kValue, true).
        set(
            agent,
            o.unbind().into_object(),
            pk,
            k_value.unbind(),
            true,
            gc.reborrow(),
        )
        .unbind()?;
        o = scoped_o.get(agent).bind(gc.nogc());
    }

    // 5. Assert: values is now an empty List.
    // 6. Return unused.
    Ok(())
}

/// ### [23.2.5.1.5 InitializeTypedArrayFromArrayLike ( O, arrayLike )](https://tc39.es/ecma262/#sec-initializetypedarrayfromarraylike)
///
/// The abstract operation InitializeTypedArrayFromArrayLike takes arguments O
/// (a TypedArray) and arrayLike (an Object, but not a TypedArray or an
/// ArrayBuffer) and returns either a normal completion containing unused or a
/// throw completion.
pub(crate) fn initialize_typed_array_from_array_like<'a, T: Viewable>(
    agent: &mut Agent,
    o: Scoped<TypedArray>,
    array_like: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Let len be ? LengthOfArrayLike(arrayLike).
    let len = length_of_array_like(agent, array_like, gc.reborrow()).unbind()? as usize;

    // 2. Perform ? AllocateTypedArrayBuffer(O, len).
    allocate_typed_array_buffer::<T>(agent, o.get(agent), len, gc.nogc()).unbind()?;

    // 3. Let k be 0.
    let mut k = 0;
    // 4. Repeat, while k < len,
    while k < len {
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk = PropertyKey::from(SmallInteger::try_from(k as i64).unwrap());
        // b. Let kValue be ? Get(arrayLike, Pk).
        let k_value = get(agent, array_like, pk, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // c. Perform ? Set(O, Pk, kValue, true).
        set(
            agent,
            o.get(agent).into_object(),
            pk,
            k_value.unbind(),
            true,
            gc.reborrow(),
        )
        .unbind()?;
        // d. Set k to k + 1.
        k += 1;
    }

    // 5. Return unused.
    Ok(())
}

/// ### [23.2.5.1.6 AllocateTypedArrayBuffer ( O, length )](https://tc39.es/ecma262/#sec-allocatetypedarraybuffer)
///
/// The abstract operation AllocateTypedArrayBuffer takes arguments O (a
/// TypedArray) and length (a non-negative integer) and returns either a normal
/// completion containing unused or a throw completion. It allocates and
/// associates an ArrayBuffer with O.
pub(crate) fn allocate_typed_array_buffer<'a, T: Viewable>(
    agent: &mut Agent,
    o: TypedArray,
    length: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Assert: O.[[ViewedArrayBuffer]] is undefined.
    // 2. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<T>();

    // 3. Let byteLength be elementSize × length.
    let byte_length = element_size * length;

    // 4. Let data be ? AllocateArrayBuffer(%ArrayBuffer%, byteLength).
    let array_buffer_constructor = agent
        .current_realm_record()
        .intrinsics()
        .array_buffer()
        .bind(gc);
    let data = allocate_array_buffer(
        agent,
        array_buffer_constructor.into_function(),
        byte_length as u64,
        None,
        gc,
    )?;

    let o_heap_data = &mut agent[o];

    let heap_byte_length = byte_length.into();
    let heap_array_length = length.into();

    // 5. Set O.[[ViewedArrayBuffer]] to data.
    o_heap_data.viewed_array_buffer = data.unbind();
    // 6. Set O.[[ByteLength]] to byteLength.
    o_heap_data.byte_length = heap_byte_length;
    // 7. Set O.[[ByteOffset]] to 0.
    o_heap_data.byte_offset = 0.into();
    // 8. Set O.[[ArrayLength]] to length.
    o_heap_data.array_length = heap_array_length;

    if heap_byte_length.is_overflowing() {
        o.set_overflowing_byte_length(agent, byte_length);
        // Note: if byte length doesn't overflow then array length cannot
        // overflow either.
        if heap_array_length.is_overflowing() {
            o.set_overflowing_array_length(agent, length);
        }
    }

    // 9. Return unused.
    Ok(())
}

/// ### [23.2.4.2 TypedArrayCreateFromConstructor ( constructor, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-typedarraycreatefromconstructor)
///
/// ### NOTE
/// This method implements steps 2 onwards of the TypedArrayCreateFromConstructor abstract operation.
fn typed_array_create_from_constructor_internal<'a>(
    agent: &mut Agent,
    new_typed_array: Object<'_>,
    length: Option<i64>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    // 2. Let taRecord be ? ValidateTypedArray(newTypedArray, seq-cst).
    let ta_record =
        validate_typed_array(agent, new_typed_array.into_value(), Ordering::SeqCst, gc)?;
    let o = ta_record.object;
    // 3. If the number of elements in argumentList is 1 and argumentList[0] is a Number, then
    if let Some(first_arg) = length {
        // a. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
        if with_typed_array_viewable!(o, is_typed_array_out_of_bounds::<T>(agent, &ta_record, gc)) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "TypedArray out of bounds",
                gc,
            ));
        }
        // b. Let length be TypedArrayLength(taRecord).
        let len =
            with_typed_array_viewable!(o, typed_array_length::<T>(agent, &ta_record, gc)) as i64;
        // c. If length < ℝ(argumentList[0]), throw a TypeError exception.
        if len < first_arg {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "TypedArray out of bounds",
                gc,
            ));
        };
    }
    // 4. Return newTypedArray.
    Ok(o)
}

/// ### [23.2.4.2 TypedArrayCreateFromConstructor ( constructor, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-typedarraycreatefromconstructor)
/// The abstract operation TypedArrayCreateFromConstructor takes arguments constructor (a constructor)
/// and argumentList (a List of ECMAScript language values)
/// and returns either a normal completion containing a TypedArray or a throw completion.
/// It is used to specify the creation of a new TypedArray using a constructor function.
pub(crate) fn typed_array_create_from_constructor_with_length<'a>(
    agent: &mut Agent,
    constructor: Function,
    length: i64,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let constructor = constructor.bind(gc.nogc());
    // 1. Let newTypedArray be ? Construct(constructor, argumentList).
    let new_typed_array = construct(
        agent,
        constructor.unbind(),
        Some(ArgumentsList::from_mut_value(
            &mut Value::try_from(length).unwrap(),
        )),
        None,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    typed_array_create_from_constructor_internal(
        agent,
        new_typed_array.unbind(),
        Some(length),
        gc.into_nogc(),
    )
}

/// ### [23.2.4.2 TypedArrayCreateFromConstructor ( constructor, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-typedarraycreatefromconstructor)
/// The abstract operation TypedArrayCreateFromConstructor takes arguments constructor (a constructor)
/// and argumentList (a List of ECMAScript language values)
/// and returns either a normal completion containing a TypedArray or a throw completion.
/// It is used to specify the creation of a new TypedArray using a constructor function.
pub(crate) fn typed_array_create_from_constructor_with_buffer<'a>(
    agent: &mut Agent,
    constructor: Function,
    array_buffer: ArrayBuffer,
    byte_offset: i64,
    length: Option<i64>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    let constructor = constructor.bind(gc.nogc());
    let array_buffer = array_buffer.bind(gc.nogc());
    let args: &mut [Value] = if let Some(length) = length {
        &mut [
            array_buffer.into_value().unbind(),
            Value::try_from(byte_offset).unwrap(),
            Value::try_from(length).unwrap(),
        ]
    } else {
        &mut [
            array_buffer.into_value().unbind(),
            Value::try_from(byte_offset).unwrap(),
        ]
    };
    // 1. Let newTypedArray be ? Construct(constructor, argumentList).
    let new_typed_array = construct(
        agent,
        constructor.unbind(),
        Some(ArgumentsList::from_mut_slice(args)),
        None,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    typed_array_create_from_constructor_internal(
        agent,
        new_typed_array.unbind(),
        length,
        gc.into_nogc(),
    )
}

/// ### [23.2.4.3 TypedArrayCreateSameType ( exemplar, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-typedarray-create-same-type)
/// The abstract operation TypedArrayCreateSameType takes arguments exemplar (a TypedArray)
/// and argumentList (a List of ECMAScript language values) and returns either
/// a normal completion containing a TypedArray or a throw completion.
/// It is used to specify the creation of a new TypedArray using a constructor function that is derived from exemplar.
/// Unlike TypedArraySpeciesCreate, which can construct custom TypedArray subclasses through the use of %Symbol.species%,
/// this operation always uses one of the built-in TypedArray constructors.
pub(crate) fn typed_array_create_same_type<'a>(
    agent: &mut Agent,
    exemplar: TypedArray,
    length: i64,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    // 1. Let constructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let constructor = match exemplar {
        TypedArray::Int8Array(_) => agent.current_realm_record().intrinsics().int8_array(),
        TypedArray::Uint8Array(_) => agent.current_realm_record().intrinsics().uint8_array(),
        TypedArray::Uint8ClampedArray(_) => agent
            .current_realm_record()
            .intrinsics()
            .uint8_clamped_array(),
        TypedArray::Int16Array(_) => agent.current_realm_record().intrinsics().int16_array(),
        TypedArray::Uint16Array(_) => agent.current_realm_record().intrinsics().uint16_array(),
        TypedArray::Int32Array(_) => agent.current_realm_record().intrinsics().int32_array(),
        TypedArray::Uint32Array(_) => agent.current_realm_record().intrinsics().uint32_array(),
        TypedArray::BigInt64Array(_) => agent.current_realm_record().intrinsics().big_int64_array(),
        TypedArray::BigUint64Array(_) => {
            agent.current_realm_record().intrinsics().big_uint64_array()
        }
        #[cfg(feature = "proposal-float16array")]
        TypedArray::Float16Array(_) => agent.current_realm_record().intrinsics().float16_array(),
        TypedArray::Float32Array(_) => agent.current_realm_record().intrinsics().float32_array(),
        TypedArray::Float64Array(_) => agent.current_realm_record().intrinsics().float64_array(),
    };
    let constructor = constructor.bind(gc.nogc());
    // 2. Let result be ? TypedArrayCreateFromConstructor(constructor, argumentList).
    let result = typed_array_create_from_constructor_with_length(
        agent,
        constructor.into_function().unbind(),
        length,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. Assert: result has [[TypedArrayName]] and [[ContentType]] internal slots.
    // 4. Assert: result.[[ContentType]] is exemplar.[[ContentType]].
    // 5. Return result.
    Ok(result.unbind())
}

fn intrinsic_default_constructor<T: Viewable>(agent: &Agent) -> BuiltinFunction<'static> {
    {
        if TypeId::of::<T>() == TypeId::of::<i8>() {
            agent.current_realm_record().intrinsics().int8_array()
        } else if TypeId::of::<T>() == TypeId::of::<u8>() {
            agent.current_realm_record().intrinsics().uint8_array()
        } else if TypeId::of::<T>() == TypeId::of::<U8Clamped>() {
            agent
                .current_realm_record()
                .intrinsics()
                .uint8_clamped_array()
        } else if TypeId::of::<T>() == TypeId::of::<i16>() {
            agent.current_realm_record().intrinsics().int16_array()
        } else if TypeId::of::<T>() == TypeId::of::<u16>() {
            agent.current_realm_record().intrinsics().uint16_array()
        } else if TypeId::of::<T>() == TypeId::of::<i32>() {
            agent.current_realm_record().intrinsics().int32_array()
        } else if TypeId::of::<T>() == TypeId::of::<u32>() {
            agent.current_realm_record().intrinsics().uint32_array()
        } else if TypeId::of::<T>() == TypeId::of::<i64>() {
            agent.current_realm_record().intrinsics().big_int64_array()
        } else if TypeId::of::<T>() == TypeId::of::<u64>() {
            agent.current_realm_record().intrinsics().big_uint64_array()
        } else if TypeId::of::<T>() == TypeId::of::<f32>() {
            agent.current_realm_record().intrinsics().float32_array()
        } else if TypeId::of::<T>() == TypeId::of::<f64>() {
            agent.current_realm_record().intrinsics().float64_array()
        } else {
            #[cfg(feature = "proposal-float16array")]
            if TypeId::of::<T>() == TypeId::of::<f16>() {
                return agent.current_realm_record().intrinsics().float16_array();
            }
            unreachable!()
        }
    }
}

fn has_matching_content_type<T: Viewable>(result: TypedArray) -> bool {
    let is_bigint = T::IS_BIGINT;
    match result {
        TypedArray::Int8Array(_)
        | TypedArray::Uint8Array(_)
        | TypedArray::Uint8ClampedArray(_)
        | TypedArray::Int16Array(_)
        | TypedArray::Uint16Array(_)
        | TypedArray::Int32Array(_)
        | TypedArray::Uint32Array(_)
        | TypedArray::Float32Array(_)
        | TypedArray::Float64Array(_) => !is_bigint,
        TypedArray::BigInt64Array(_) | TypedArray::BigUint64Array(_) => is_bigint,
        #[cfg(feature = "proposal-float16array")]
        TypedArray::Float16Array(_) => !is_bigint,
    }
}

/// ### [23.2.4.1 TypedArraySpeciesCreate ( exemplar, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#typedarray-species-create)
pub(crate) fn typed_array_species_create_with_length<'a, T: Viewable>(
    agent: &mut Agent,
    exemplar: TypedArray,
    length: i64,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    // 1. Let defaultConstructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let default_constructor = intrinsic_default_constructor::<T>(agent).bind(gc.nogc());
    // 2. Let constructor be ? SpeciesConstructor(exemplar, defaultConstructor).
    let constructor = species_constructor(
        agent,
        exemplar.into_object(),
        default_constructor.into_function().unbind(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. Let result be ? TypedArrayCreateFromConstructor(constructor, argumentList).
    let result = typed_array_create_from_constructor_with_length(
        agent,
        constructor.unbind(),
        length,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 4. Assert: result has [[TypedArrayName]] and [[ContentType]] internal slots.
    // 5. If result.[[ContentType]] is not exemplar.[[ContentType]], throw a TypeError exception.
    let is_type_match = has_matching_content_type::<T>(result);
    if !is_type_match {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray species did not match exemplar",
            gc.into_nogc(),
        ));
    }
    // 6. Return result.
    Ok(result.unbind())
}

pub(crate) fn typed_array_species_create_with_buffer<'a, T: Viewable>(
    agent: &mut Agent,
    exemplar: TypedArray,
    array_buffer: ArrayBuffer,
    byte_offset: i64,
    length: Option<i64>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    // 1. Let defaultConstructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let default_constructor = intrinsic_default_constructor::<T>(agent).bind(gc.nogc());
    // 2. Let constructor be ? SpeciesConstructor(exemplar, defaultConstructor).
    let constructor = species_constructor(
        agent,
        exemplar.into_object(),
        default_constructor.into_function().unbind(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. Let result be ? TypedArrayCreateFromConstructor(constructor, argumentList).
    let result = typed_array_create_from_constructor_with_buffer(
        agent,
        constructor.unbind(),
        array_buffer,
        byte_offset,
        length,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 4. Assert: result has [[TypedArrayName]] and [[ContentType]] internal slots.
    // 5. If result.[[ContentType]] is not exemplar.[[ContentType]], throw a TypeError exception.
    let is_type_match = has_matching_content_type::<T>(result);
    if !is_type_match {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "can't convert BigInt to number",
            gc.into_nogc(),
        ));
    }
    // 6. Return result.
    Ok(result.unbind())
}

/// [23.2.3.26.1 SetTypedArrayFromTypedArray ( target, targetOffset, source )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-settypedarrayfromtypedarray)
/// The abstract operation SetTypedArrayFromTypedArray takes arguments target
/// (a TypedArray), targetOffset (a non-negative integer or +∞), and source
/// (a TypedArray) and returns either a normal completion containing unused
/// or a throw completion. It sets multiple values in target, starting at index
/// targetOffset, reading the values from source.
pub(crate) fn set_typed_array_from_typed_array<'a, TargetType: Viewable, SrcType: Viewable>(
    agent: &mut Agent,
    target: TypedArray,
    target_offset: IntegerOrInfinity,
    source: TypedArray,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let target = target.bind(gc);
    let source = source.bind(gc);
    // 1. Let targetBuffer be target.[[ViewedArrayBuffer]].
    let target_buffer = target.get_viewed_array_buffer(agent, gc);
    // 2. Let targetRecord be MakeTypedArrayWithBufferWitnessRecord(target, seq-cst).
    let target_record =
        make_typed_array_with_buffer_witness_record(agent, target, Ordering::SeqCst, gc);
    // 3. If IsTypedArrayOutOfBounds(targetRecord) is true, throw a TypeError exception.
    if is_typed_array_out_of_bounds::<TargetType>(agent, &target_record, gc) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc,
        ));
    };
    // 4. Let targetLength be TypedArrayLength(targetRecord).
    let target_length = typed_array_length::<TargetType>(agent, &target_record, gc);
    // 5. Let srcBuffer be source.[[ViewedArrayBuffer]].
    let mut src_buffer = source.get_viewed_array_buffer(agent, gc);
    // 6. Let srcRecord be MakeTypedArrayWithBufferWitnessRecord(source, seq-cst).
    let src_record =
        make_typed_array_with_buffer_witness_record(agent, source, Ordering::SeqCst, gc);
    // 7. If IsTypedArrayOutOfBounds(srcRecord) is true, throw a TypeError exception.
    if is_typed_array_out_of_bounds::<SrcType>(agent, &src_record, gc) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc,
        ));
    }
    // 8. Let srcLength be TypedArrayLength(srcRecord).
    let src_length = typed_array_length::<SrcType>(agent, &src_record, gc);
    // 9. Let targetType be TypedArrayElementType(target).
    // 10. Let targetElementSize be TypedArrayElementSize(target).
    let target_element_size = size_of::<TargetType>();
    // 11. Let targetByteOffset be target.[[ByteOffset]].
    let target_byte_offset = target.byte_offset(agent);
    // 12. Let srcType be TypedArrayElementType(source).
    // 13. Let srcElementSize be TypedArrayElementSize(source).
    // 14. Let srcByteOffset be source.[[ByteOffset]].
    let src_byte_offset = source.byte_offset(agent);
    // 15. If targetOffset = +∞, throw a RangeError exception.
    if target_offset.is_pos_infinity() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "count must be less than infinity",
            gc,
        ));
    };
    // 16. If srcLength + targetOffset > targetLength, throw a RangeError exception.
    let target_offset = target_offset.into_i64() as u64;
    if src_length as u64 + target_offset > target_length as u64 {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "source length out of target bounds",
            gc,
        ));
    };
    let target_offset = target_offset as usize;
    // 17. If target.[[ContentType]] is not source.[[ContentType]], throw a TypeError exception.
    let is_type_match = has_matching_content_type::<TargetType>(source);
    if !is_type_match {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray species did not match source",
            gc,
        ));
    };
    // 18. If IsSharedArrayBuffer(srcBuffer) is true,
    //     IsSharedArrayBuffer(targetBuffer) is true, and
    //     srcBuffer.[[ArrayBufferData]] is targetBuffer.[[ArrayBufferData]],
    //     let sameSharedArrayBuffer be true; otherwise, let
    //     sameSharedArrayBuffer be false.
    // 19. If SameValue(srcBuffer, targetBuffer) is true or
    //     sameSharedArrayBuffer is true, then
    let src_byte_index = if src_buffer == target_buffer {
        // a. Let srcByteLength be TypedArrayByteLength(srcRecord).
        let src_byte_length = typed_array_byte_length::<SrcType>(agent, &src_record, gc);
        // b. Set srcBuffer to
        //    ? CloneArrayBuffer(srcBuffer, srcByteOffset, srcByteLength).
        src_buffer = clone_array_buffer(agent, src_buffer, src_byte_offset, src_byte_length, gc)
            .unbind()?
            .bind(gc);
        // c. Let srcByteIndex be 0.
        0
    } else {
        src_byte_offset
    };
    debug_assert_ne!(src_buffer, target_buffer);
    debug_assert_ne!(
        src_buffer.as_slice(agent).as_ptr(),
        target_buffer.as_slice(agent).as_ptr()
    );
    // 21. Let targetByteIndex be (targetOffset × targetElementSize) + targetByteOffset.
    let target_byte_index = (target_offset * target_element_size) + target_byte_offset;
    // 22. Let limit be targetByteIndex + (targetElementSize × srcLength).
    let limit = target_byte_index + (target_element_size * src_length);
    // 23. If srcType is targetType, then
    if core::any::TypeId::of::<SrcType>() == core::any::TypeId::of::<TargetType>() {
        // a. NOTE: The transfer must be performed in a manner that preserves
        //    the bit-level encoding of the source data.
        // Repeat, while targetByteIndex < limit,
        // i. Let value be GetValueFromBuffer(srcBuffer, srcByteIndex, uint8,
        //    true, unordered).
        // ii. Perform SetValueInBuffer(targetBuffer, targetByteIndex, uint8,
        //     value, true, unordered).
        let (target_slice, src_slice) = split_typed_array_buffers::<SrcType>(
            agent,
            target_buffer,
            target_byte_index,
            src_buffer,
            src_byte_index,
            limit,
        );
        target_slice.copy_from_slice(src_slice);
        // iii. Set srcByteIndex to srcByteIndex + 1.
        // iv. Set targetByteIndex to targetByteIndex + 1.
    } else {
        // 24. Else,
        //  a. Repeat, while targetByteIndex < limit,
        //  i. Let value be GetValueFromBuffer(srcBuffer, srcByteIndex, srcType, true, unordered).
        //  ii. Perform SetValueInBuffer(targetBuffer, targetByteIndex, targetType, value, true, unordered).
        let target_slice = byte_slice_to_viewable_mut::<TargetType>(
            target_buffer.as_mut_slice(agent),
            target_byte_index,
            limit,
        );
        let target_ptr = target_slice.as_mut_ptr();
        let target_len = target_slice.len();
        let src_slice = byte_slice_to_viewable::<SrcType>(
            src_buffer.as_slice(agent),
            src_byte_index,
            // Note: source buffer is limited by the target buffer length.
            src_byte_index + target_len * core::mem::size_of::<SrcType>(),
        );
        // SAFETY: Confirmed beforehand that the two ArrayBuffers are in separate memory regions.
        let target_slice = unsafe { std::slice::from_raw_parts_mut(target_ptr, target_len) };
        copy_between_different_type_typed_arrays::<SrcType, TargetType>(src_slice, target_slice);
        //  iii. Set srcByteIndex to srcByteIndex + srcElementSize.
        //  iv. Set targetByteIndex to targetByteIndex + targetElementSize.
    }
    // 25. Return unused.
    Ok(())
}

/// ### [23.2.3.26.2 SetTypedArrayFromArrayLike ( target, targetOffset, source )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-settypedarrayfromarraylike)
/// The abstract operation SetTypedArrayFromArrayLike takes arguments target
/// (a TypedArray), targetOffset (a non-negative integer or +∞), and source
/// (an ECMAScript language value, but not a TypedArray) and returns either
/// a normal completion containing unused or a throw completion. It sets
/// multiple values in target, starting at index targetOffset, reading the
/// values from source.
pub(crate) fn set_typed_array_from_array_like<'a, T: Viewable>(
    agent: &mut Agent,
    target: Scoped<TypedArray>,
    target_offset: IntegerOrInfinity,
    source: Scoped<Value>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Let targetRecord be MakeTypedArrayWithBufferWitnessRecord(target, seq-cst).
    let target_record = make_typed_array_with_buffer_witness_record(
        agent,
        target.get(agent),
        Ordering::SeqCst,
        gc.nogc(),
    );
    // 2. If IsTypedArrayOutOfBounds(targetRecord) is true, throw a TypeError exception.
    if is_typed_array_out_of_bounds::<T>(agent, &target_record, gc.nogc()) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc.into_nogc(),
        ));
    };
    // 3. Let targetLength be TypedArrayLength(targetRecord).
    let target_length = typed_array_length::<T>(agent, &target_record, gc.nogc()) as u64;
    // 4. Let src be ? ToObject(source).
    let src = to_object(agent, source.get(agent), gc.nogc())
        .unbind()?
        .bind(gc.nogc());
    // SAFETY: source is not shared.
    let source = unsafe { source.replace_self(agent, src.unbind()) };
    // 5. Let srcLength be ? LengthOfArrayLike(src).
    let src_length = length_of_array_like(agent, src.unbind(), gc.reborrow()).unbind()? as u64;
    let src = source;
    // 6. If targetOffset = +∞, throw a RangeError exception.
    if target_offset.is_pos_infinity() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "count must be less than infinity",
            gc.into_nogc(),
        ));
    };
    let target_offset = target_offset.into_i64() as u64;
    // 7. If srcLength + targetOffset > targetLength, throw a RangeError exception.
    if src_length + target_offset > target_length {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "count must be less than infinity",
            gc.into_nogc(),
        ));
    };
    let target_offset = target_offset as usize;
    let src_length = src_length as usize;
    // 8. Let k be 0.
    let mut k = 0;
    // 9. Repeat, while k < srcLength,
    while k < src_length {
        // a. Let Pk be ! ToString(𝔽(k)).
        let pk = PropertyKey::Integer(k.try_into().unwrap());
        // b. Let value be ? Get(src, Pk).
        let value = get(agent, src.get(agent), pk, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // c. Let targetIndex be 𝔽(targetOffset + k).
        let target_index = target_offset + k;
        // d. Perform ? TypedArraySetElement(target, targetIndex, value).
        typed_array_set_element::<T>(
            agent,
            target.get(agent),
            target_index as i64,
            value.unbind(),
            gc.reborrow(),
        )
        .unbind()?;
        // e. Set k to k + 1.
        k += 1;
    }
    // 10. Return unused.
    Ok(())
}
