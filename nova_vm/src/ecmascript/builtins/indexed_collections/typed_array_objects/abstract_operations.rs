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
                try_species_constructor,
            },
            type_conversion::{
                IntegerOrInfinity, to_big_int, to_index, to_number, to_object, validate_index,
            },
        },
        builtins::{
            ArgumentsList, ArrayBuffer,
            array_buffer::{
                AnyArrayBuffer, ViewedArrayBufferByteLength, array_buffer_byte_length,
                clone_array_buffer, get_value_from_buffer, is_detached_buffer,
                is_fixed_length_array_buffer, set_value_in_buffer,
            },
            indexed_collections::typed_array_objects::typed_array_intrinsic_object::{
                byte_slice_to_viewable, byte_slice_to_viewable_mut,
                require_internal_slot_typed_array, split_typed_array_buffers,
            },
            ordinary::get_prototype_from_constructor,
            typed_array::{
                AnyTypedArray, GenericTypedArray, TypedArray,
                data::{TypedArrayArrayLength, TypedArrayRecord},
            },
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics,
            agent::{ExceptionType, TryError, TryResult, js_result_into_try},
        },
        types::{
            BigInt, DataBlock, Function, InternalSlots, IntoFunction, IntoNumeric, IntoObject,
            IntoValue, Number, Numeric, Object, PropertyKey, U8Clamped, Value, Viewable,
            create_byte_data_block,
        },
    },
    engine::{
        Scoped, ScopedCollection,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{Rootable, Scopable},
    },
    heap::CreateHeapData,
};

use super::typed_array_intrinsic_object::copy_between_different_type_typed_arrays;

/// Matches a TypedArray and defines a type T in the expression which
/// is the generic type of the viewable.
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
use ecmascript_atomics::Ordering;
pub(crate) use with_typed_array_viewable;

/// Matches a TypedArray and defines a type T in the expression which
/// is the generic type of the viewable.
macro_rules! match_typed_array {
    ($value:expr, $expr:expr, $as:ident) => {
        match $value {
            TypedArray::Int8Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Uint8Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Uint8ClampedArray(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Int16Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Uint16Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Int32Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Uint32Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::BigInt64Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::BigUint64Array(ta) => {
                let $as = ta;
                $expr
            }
            #[cfg(feature = "proposal-float16array")]
            TypedArray::Float16Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Float32Array(ta) => {
                let $as = ta;
                $expr
            }
            TypedArray::Float64Array(ta) => {
                let $as = ta;
                $expr
            }
        }
    };
}
pub(crate) use match_typed_array;

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
}

impl From<CachedBufferByteLength> for Option<usize> {
    fn from(val: CachedBufferByteLength) -> Self {
        if val.is_detached() { None } else { Some(val.0) }
    }
}

#[derive(Debug)]
pub(crate) struct TypedArrayWithBufferWitnessRecords<'a> {
    pub object: AnyTypedArray<'a>,
    pub cached_buffer_byte_length: CachedBufferByteLength,
}
bindable_handle!(TypedArrayWithBufferWitnessRecords);

impl<'ta> TypedArrayWithBufferWitnessRecords<'ta> {
    /// ### [10.4.5.13 IsTypedArrayOutOfBounds ( taRecord )](https://tc39.es/ecma262/#sec-istypedarrayoutofbounds)
    ///
    /// The abstract operation IsTypedArrayOutOfBounds takes argument taRecord (a
    /// TypedArray With Buffer Witness Record) and returns a Boolean. It checks if
    /// any of the object's numeric properties reference a value at an index not
    /// contained within the underlying buffer's bounds.
    pub(crate) fn is_typed_array_out_of_bounds(&self, agent: &Agent) -> bool {
        self.object
            .is_typed_array_out_of_bounds(agent, self.cached_buffer_byte_length)
    }

    /// ### [10.4.5.12 TypedArrayLength ( taRecord )](https://tc39.es/ecma262/#sec-typedarraylength)
    ///
    /// The abstract operation TypedArrayLength takes argument taRecord (a
    /// TypedArray With Buffer Witness Record) and returns a non-negative integer.
    pub(crate) fn typed_array_length(&self, agent: &Agent) -> usize {
        self.object
            .typed_array_length(agent, self.cached_buffer_byte_length)
    }
}

/// ### [10.4.5.9 MakeTypedArrayWithBufferWitnessRecord ( obj, order )](https://tc39.es/ecma262/#sec-maketypedarraywithbufferwitnessrecord)
///
/// The abstract operation MakeTypedArrayWithBufferWitnessRecord takes arguments
/// obj (a TypedArray) and order (seq-cst or unordered) and returns a TypedArray
/// With Buffer Witness Record.
pub(crate) fn make_typed_array_with_buffer_witness_record<'a>(
    agent: &Agent,
    obj: AnyTypedArray<'a>,
    order: Ordering,
) -> TypedArrayWithBufferWitnessRecords<'a> {
    // 1. Let buffer be obj.[[ViewedArrayBuffer]].
    let buffer = obj.viewed_array_buffer(agent);

    // 2. If IsDetachedBuffer(buffer) is true, then
    let byte_length = if buffer.is_detached(agent) {
        // a. Let byteLength be detached.
        CachedBufferByteLength::detached()
    } else {
        // 3. Else,
        // a. Let byteLength be ArrayBufferByteLength(buffer, order).
        CachedBufferByteLength::value(buffer.byte_length(agent, order))
    };

    // 4. Return the TypedArray With Buffer Witness Record { [[Object]]: obj, [[CachedBufferByteLength]]: byteLength }.
    TypedArrayWithBufferWitnessRecords {
        object: obj,
        cached_buffer_byte_length: byte_length,
    }
}

pub(crate) fn make_typed_array_with_buffer_witness_record_specialised<'a, O: Viewable>(
    agent: &Agent,
    obj: GenericTypedArray<'a, O>,
    order: Ordering,
) -> (GenericTypedArray<'a, O>, CachedBufferByteLength) {
    // 1. Let buffer be obj.[[ViewedArrayBuffer]].
    let buffer = obj.viewed_array_buffer(agent);

    // 2. If IsDetachedBuffer(buffer) is true, then
    let byte_length = if buffer.is_detached(agent) {
        // a. Let byteLength be detached.
        CachedBufferByteLength::detached()
    } else {
        // 3. Else,
        // a. Let byteLength be ArrayBufferByteLength(buffer, order).
        CachedBufferByteLength::value(buffer.byte_length(agent, order))
    };

    // 4. Return the TypedArray With Buffer Witness Record { [[Object]]: obj, [[CachedBufferByteLength]]: byteLength }.
    (obj, byte_length)
}

/// ### [10.4.5.10 TypedArrayCreate ( prototype )](https://tc39.es/ecma262/#sec-typedarraycreate)
///
/// The abstract operation TypedArrayCreate takes argument prototype (an Object)
/// and returns a TypedArray. It is used to specify the creation of new TypedArrays.
pub(crate) fn typed_array_create<'a, T: Viewable>(
    agent: &mut Agent,
    prototype: Option<Object<'a>>,
) -> GenericTypedArray<'a, T> {
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
    let a = TypedArrayRecord::default();

    let a = agent.heap.create(a);

    if prototype.is_some() {
        a.internal_set_prototype(agent, prototype);
    }

    a
}

/// Trait for implementing TypedArrays in their various forms without
/// duplicating code all over the place. At the core here are APIs to query the
/// byte offset, byte length, and element length of a TypedArray. After that it
/// is all just jazz.
pub(crate) trait TypedArrayAbstractOperations<'ta>: Copy + Sized {
    type ElementType: Viewable;

    /// Return `IsDetachedBuffer(O.[[ViewedArrayBuffer]])`.
    fn is_detached(self, agent: &Agent) -> bool;
    fn is_fixed_length(self, agent: &Agent) -> bool;

    /// \[\[ByteOffset]]
    fn byte_offset(self, agent: &Agent) -> usize;
    /// \[\[ByteLength]]
    fn byte_length(self, agent: &Agent) -> Option<usize>;
    /// \[\[ArrayOffset]]
    fn array_length(self, agent: &Agent) -> Option<usize>;

    /// 23.2.4.5 TypedArrayElementSize ( O )
    fn typed_array_element_size(self) -> usize;

    /// \[\[ViewedArrayBuffer]]
    fn viewed_array_buffer(self, agent: &Agent) -> AnyArrayBuffer<'ta>;
    fn get_cached_buffer_byte_length(
        self,
        agent: &Agent,
        order: Ordering,
    ) -> CachedBufferByteLength;

    /// ### [10.4.5.11 TypedArrayByteLength ( taRecord )](https://tc39.es/ecma262/#sec-typedarraybytelength)
    ///
    /// The abstract operation TypedArrayByteLength takes argument taRecord (a
    /// TypedArray With Buffer Witness Record) and returns a non-negative integer.
    fn typed_array_byte_length(
        self,
        agent: &mut Agent,
        cached_buffer_byte_length: CachedBufferByteLength,
    ) -> usize {
        // 1. If IsTypedArrayOutOfBounds(taRecord) is true, return 0.
        if self.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length) {
            return 0;
        }

        // 2. Let length be TypedArrayLength(taRecord).
        let length = self.typed_array_length(agent, cached_buffer_byte_length);

        // 3. If length = 0, return 0.
        if length == 0 {
            return 0;
        }

        // 4. Let O be taRecord.[[Object]].
        // 5. If O.[[ByteLength]] is not auto, return O.[[ByteLength]].
        if let Some(byte_length) = self.byte_length(agent) {
            return byte_length;
        }

        // 6. Let elementSize be TypedArrayElementSize(O).
        let element_size = size_of::<Self::ElementType>();
        // 7. Return length × elementSize.
        length * element_size
    }

    /// ### [10.4.5.12 TypedArrayLength ( taRecord )](https://tc39.es/ecma262/#sec-typedarraylength)
    ///
    /// The abstract operation TypedArrayLength takes argument taRecord (a
    /// TypedArray With Buffer Witness Record) and returns a non-negative integer.
    fn typed_array_length(
        self,
        agent: &Agent,
        cached_buffer_byte_length: CachedBufferByteLength,
    ) -> usize {
        // 1. Assert: IsTypedArrayOutOfBounds(taRecord) is false.
        debug_assert!(!self.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length));

        // 3. If O.[[ArrayLength]] is not auto, return O.[[ArrayLength]].
        if let Some(array_length) = self.array_length(agent) {
            return array_length;
        }

        // 4. Assert: IsFixedLengthArrayBuffer(O.[[ViewedArrayBuffer]]) is false.
        debug_assert!(!self.is_fixed_length(agent));

        // 5. Let byteOffset be O.[[ByteOffset]].
        let byte_offset = self.byte_offset(agent);

        // 6. Let elementSize be TypedArrayElementSize(O).
        let element_size = size_of::<Self::ElementType>();

        // 7. Let byteLength be taRecord.[[CachedBufferByteLength]].
        // 8. Assert: byteLength is not detached.
        debug_assert!(!cached_buffer_byte_length.is_detached());
        let byte_length = cached_buffer_byte_length.0;

        // 9. Return floor((byteLength - byteOffset) / elementSize).
        (byte_length - byte_offset) / element_size
    }

    /// ### [10.4.5.13 IsTypedArrayOutOfBounds ( taRecord )](https://tc39.es/ecma262/#sec-istypedarrayoutofbounds)
    ///
    /// The abstract operation IsTypedArrayOutOfBounds takes argument taRecord (a
    /// TypedArray With Buffer Witness Record) and returns a Boolean. It checks if
    /// any of the object's numeric properties reference a value at an index not
    /// contained within the underlying buffer's bounds.
    fn is_typed_array_out_of_bounds(
        self,
        agent: &Agent,
        cached_buffer_byte_length: CachedBufferByteLength,
    ) -> bool {
        // 3. Assert: IsDetachedBuffer(O.[[ViewedArrayBuffer]]) is true if and only if bufferByteLength is detached.
        assert_eq!(
            self.is_detached(agent),
            cached_buffer_byte_length.is_detached()
        );

        // 4. If bufferByteLength is detached, return true.
        let Some(buffer_byte_length) = cached_buffer_byte_length.into() else {
            return true;
        };

        // 5. Let byteOffsetStart be O.[[ByteOffset]].
        let byte_offset_start = self.byte_offset(agent);

        // 6. If O.[[ArrayLength]] is auto, then
        let byte_offset_end = if let Some(array_length) = self.array_length(agent) {
            // 7. Else,
            // a. Let elementSize be TypedArrayElementSize(O).
            let element_size = size_of::<Self::ElementType>();
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
    fn is_typed_array_fixed_length(self, agent: &Agent) -> bool {
        // 1. If O.[[ArrayLength]] is auto, return false.
        if self.array_length(agent).is_none() {
            false
        } else {
            // 2. Let buffer be O.[[ViewedArrayBuffer]].
            let buffer = self.viewed_array_buffer(agent);
            // 3. If IsFixedLengthArrayBuffer(buffer) is false and IsSharedArrayBuffer(buffer) is false, return false.
            if buffer.is_resizable(agent) && !buffer.is_shared() {
                false
            } else {
                // 4. Return true.
                true
            }
        }
    }

    /// ### [10.4.5.16 IsValidIntegerIndex ( O, index )](https://tc39.es/ecma262/#sec-isvalidintegerindex)
    ///
    /// The abstract operation IsValidIntegerIndex takes arguments O (a TypedArray)
    /// and index (a Number) and returns a Boolean.
    fn is_valid_integer_index(self, agent: &Agent, index: i64) -> Option<usize> {
        // 1. If IsDetachedBuffer(O.[[ViewedArrayBuffer]]) is true, return false.
        if self.is_detached(agent) {
            return None;
        }
        // 2. If index is not an integral Number, return false.
        // 3. If index is -0𝔽 or index < -0𝔽, return false.
        if index < 0 {
            return None;
        }
        let index = index as usize;
        // 4. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, unordered).
        let cached_buffer_byte_length: CachedBufferByteLength =
            self.get_cached_buffer_byte_length(agent, Ordering::Unordered);
        // make_typed_array_with_buffer_witness_record_specialised(agent, o, Ordering::Unordered);
        // 5. NOTE: Bounds checking is not a synchronizing operation when O's
        //    backing buffer is a growable SharedArrayBuffer.
        // 6. If IsTypedArrayOutOfBounds(taRecord) is true, return false.
        if self.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length) {
            return None;
        }
        // 7. Let length be TypedArrayLength(taRecord).
        let length = self.typed_array_length(agent, cached_buffer_byte_length);
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
    fn typed_array_get_element<'gc>(
        self,
        agent: &mut Agent,
        index: i64,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<Numeric<'gc>> {
        // 1. If IsValidIntegerIndex(O, index) is false, return undefined.
        let index = self.is_valid_integer_index(agent, index)?;
        // 2. Let offset be O.[[ByteOffset]].
        let offset = self.byte_offset(agent);
        // 3. Let elementSize be TypedArrayElementSize(O).
        let element_size = core::mem::size_of::<Self::ElementType>();
        // 4. Let byteIndexInBuffer be (ℝ(index) × elementSize) + offset.
        let byte_index_in_buffer = (index * element_size) + offset;
        // 5. Let elementType be TypedArrayElementType(O).
        // 6. Return GetValueFromBuffer(O.[[ViewedArrayBuffer]], byteIndexInBuffer, elementType, true, unordered).
        Some(get_value_from_buffer::<Self::ElementType>(
            agent,
            self.viewed_array_buffer(agent).into(),
            byte_index_in_buffer,
            true,
            Ordering::Unordered,
            None,
            gc,
        ))
    }

    /// ### [10.4.5.18 TypedArraySetElement ( O, index, value )](https://tc39.es/ecma262/#sec-typedarraysetelement)
    ///
    /// The abstract operation TypedArraySetElement takes arguments O (a
    /// TypedArray), index (a Number), and value (an ECMAScript language value) and
    /// returns either a normal completion containing unused or a throw completion.
    ///
    /// > NOTE 1: This operation always appears to succeed, but it has no
    /// > effect when attempting to write past the end of a TypedArray or to a
    /// > TypedArray which is backed by a detached ArrayBuffer.
    ///
    /// > NOTE 2: This operation implements steps 3 onwards; steps 1 and 2 must
    /// > be done separately.
    fn typed_array_set_element(self, agent: &mut Agent, index: i64, num_value: Numeric) {
        // 3. If IsValidIntegerIndex(O, index) is true, then
        if let Some(index) = self.is_valid_integer_index(agent, index) {
            // a. Let offset be O.[[ByteOffset]].
            let offset = self.byte_offset(agent);
            // b. Let elementSize be TypedArrayElementSize(O).
            let element_size = core::mem::size_of::<Self::ElementType>();
            // c. Let byteIndexInBuffer be (ℝ(index) × elementSize) + offset.
            let byte_index_in_buffer = index * element_size + offset;
            // d. Let elementType be TypedArrayElementType(O).
            // e. Perform SetValueInBuffer(O.[[ViewedArrayBuffer]], byteIndexInBuffer, elementType, numValue, true, unordered).
            set_value_in_buffer::<Self::ElementType>(
                agent,
                self.viewed_array_buffer(agent),
                byte_index_in_buffer,
                num_value,
                true,
                Ordering::Unordered,
                None,
            );
        }
        // 4. Return UNUSED.
    }

    /// ### 23.2.3.6 %TypedArray%.prototype.copyWithin ( target, start \[ , end \] )
    fn copy_within(self, agent: &mut Agent, start_index: usize, target_index: usize, count: usize);

    /// ### 23.2.3.9 %TypedArray%.prototype.fill ( value \[ , start \[ , end \] \] )
    fn fill(self, agent: &mut Agent, value: Numeric, start_index: usize, count: usize);

    /// ### 23.2.3.10 %TypedArray%.prototype.filter ( callback \[ , thisArg \] )
    fn filter<'gc>(
        self,
        agent: &mut Agent,
        callback: Function,
        this_arg: Value,
        len: usize,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>>;

    fn search<const ASCENDING: bool>(
        self,
        agent: &mut Agent,
        search_element: Value,
        start: usize,
        end: usize,
    ) -> Option<usize>;

    fn map<'gc>(
        self,
        agent: &mut Agent,
        callback: Function,
        this_arg: Value,
        len: usize,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>>;

    fn reverse(self, agent: &mut Agent, len: usize);

    /// [23.2.3.26.2 SetTypedArrayFromTypedArray ( target, targetOffset, source )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-settypedarrayfromtypedarray)
    ///
    /// The abstract operation SetTypedArrayFromTypedArray takes arguments
    /// target (a TypedArray), targetOffset (a non-negative integer or +∞), and
    /// source (a TypedArray) and returns either a normal completion containing
    /// unused or a throw completion. It sets multiple values in target,
    /// starting at index targetOffset, reading the values from source.
    fn set_from_typed_array<'gc>(
        self,
        agent: &mut Agent,
        target_offset: usize,
        source: AnyTypedArray,
        src_length: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, ()>;
}

/// Matches a TypedArray and defines a type T in the expression which
/// is the generic type of the viewable.
macro_rules! validate_typed_array_macro {
    ($agent:expr, $value:expr, $ta:ident, $order:expr, $gc:expr, $expr:expr) => {
        match $value {
            Value::Int8Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Uint8Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Uint8ClampedArray($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Int16Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Uint16Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Int32Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Uint32Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::BigInt64Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::BigUint64Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "proposal-float16array")]
            Value::Float16Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Float32Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            Value::Float64Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt8Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint8Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint8ClampedArray($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt16Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint16Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedInt32Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedUint32Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedBigInt64Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedBigUint64Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
            Value::SharedFloat16Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedFloat32Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedFloat64Array($ta) => {
                // 3. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(O, order).
                let cached_buffer_byte_length = $ta.get_cached_buffer_byte_length($agent, $order);
                // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if $ta.is_typed_array_out_of_bounds($agent, cached_buffer_byte_length) {
                    return Err($agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        $gc.into_nogc(),
                    ));
                }

                // 5. Return taRecord.
                let $ta = ($ta, cached_buffer_byte_length);
                $expr
            }
            _ => {
                return Err($agent.throw_exception_with_static_message(
                    crate::ecmascript::execution::agent::ExceptionType::TypeError,
                    "Expected this to be TypedArray",
                    $gc.into_nogc(),
                ))
            }
        }
    };
}
pub(crate) use validate_typed_array_macro;

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
    let ta_record = make_typed_array_with_buffer_witness_record(agent, o, order);
    // 4. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
    if ta_record
        .object
        .is_typed_array_out_of_bounds(agent, ta_record.cached_buffer_byte_length)
    {
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
    length: Option<usize>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, GenericTypedArray<'a, T>> {
    let new_target = new_target.bind(gc.nogc());
    // 1. Let proto be ? GetPrototypeFromConstructor(newTarget, defaultProto).
    let proto = get_prototype_from_constructor(agent, new_target.unbind(), T::PROTO, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // 2. Let obj be TypedArrayCreate(proto).
    let obj = typed_array_create::<T>(agent, proto);

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
    o: GenericTypedArray<'a, O>,
    src_array: GenericTypedArray<'a, Src>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Let srcData be srcArray.[[ViewedArrayBuffer]].
    let src_data = src_array.viewed_array_buffer(agent);

    // 2. Let elementType be TypedArrayElementType(O).
    // 3. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<O>();

    // 4. Let srcType be TypedArrayElementType(srcArray).
    // 5. Let srcElementSize be TypedArrayElementSize(srcArray).
    let src_element_size = size_of::<Src>();

    // 6. Let srcByteOffset be srcArray.[[ByteOffset]].
    let src_byte_offset = src_array.byte_offset(agent);

    // 7. Let srcRecord be MakeTypedArrayWithBufferWitnessRecord(srcArray, seq-cst).
    let cached_src_byte_length = src_array.get_cached_buffer_byte_length(agent, Ordering::SeqCst);

    // 8. If IsTypedArrayOutOfBounds(srcRecord) is true, throw a TypeError exception.
    if src_array.is_typed_array_out_of_bounds(agent, cached_src_byte_length) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc,
        ));
    }

    // 9. Let elementLength be TypedArrayLength(srcRecord).
    let element_length = src_array.typed_array_length(agent, cached_src_byte_length);

    // 10. Let byteLength be elementSize × elementLength.
    let byte_length = element_size * element_length;

    // 11. If elementType is srcType, then
    let data = if O::PROTO == Src::PROTO {
        // a. Let data be ? CloneArrayBuffer(srcData, srcByteOffset, byteLength).
        clone_array_buffer(agent, src_data, src_byte_offset, byte_length, gc)?
    } else {
        // 12. Else,
        // a. Let data be ? AllocateArrayBuffer(%ArrayBuffer%, byteLength).
        let data = ArrayBuffer::new(agent, byte_length, gc)?;

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
                src_data.into(),
                src_byte_index,
                true,
                Ordering::Unordered,
                None,
                gc,
            );

            // ii. Perform SetValueInBuffer(data, targetByteIndex, elementType, value, true, unordered).
            set_value_in_buffer::<O>(
                agent,
                data.into(),
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

    let heap_byte_length = byte_length.into();
    let heap_array_length = element_length.into();

    // 13. Set O.[[ViewedArrayBuffer]] to data.
    // 14. Set O.[[ByteLength]] to byteLength.
    // 15. Set O.[[ByteOffset]] to 0.
    // 16. Set O.[[ArrayLength]] to elementLength.
    // SAFETY: this method is for initialising O.
    unsafe { o.initialise_data(agent, data, heap_byte_length, 0.into(), heap_array_length) };

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
    scoped_o: Scoped<GenericTypedArray<T>>,
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
    let buffer_is_fixed_length = is_fixed_length_array_buffer(agent, buffer.into());

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
        // c. Set O.[[ArrayLength]] to auto.
        // 10. Set O.[[ViewedArrayBuffer]] to buffer.
        // 11. Set O.[[ByteOffset]] to offset.
        // SAFETY: We are initialising O.
        unsafe {
            o.initialise_data(
                agent,
                buffer,
                ViewedArrayBufferByteLength::auto(),
                heap_byte_offset,
                TypedArrayArrayLength::auto(),
            )
        };

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
        // d. Set O.[[ArrayLength]] to newByteLength / elementSize.
        // 10. Set O.[[ViewedArrayBuffer]] to buffer.
        // 11. Set O.[[ByteOffset]] to offset.
        // SAFETY: We're initialising O.
        unsafe {
            o.initialise_data(
                agent,
                buffer,
                heap_byte_length,
                heap_byte_offset,
                heap_array_length,
            )
        };

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
    scoped_o: Scoped<GenericTypedArray<T>>,
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
    o: Scoped<GenericTypedArray<T>>,
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
    o: GenericTypedArray<'a, T>,
    length: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Assert: O.[[ViewedArrayBuffer]] is undefined.
    // 2. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<T>();

    // 3. Let byteLength be elementSize × length.
    let byte_length = element_size * length;

    // 4. Let data be ? AllocateArrayBuffer(%ArrayBuffer%, byteLength).
    let data = ArrayBuffer::new(agent, byte_length, gc)?;

    let heap_byte_length = byte_length.into();
    let heap_array_length = length.into();

    // 5. Set O.[[ViewedArrayBuffer]] to data.
    // 6. Set O.[[ByteLength]] to byteLength.
    // 7. Set O.[[ByteOffset]] to 0.
    // 8. Set O.[[ArrayLength]] to length.
    // SAFETY: We're initialising O.
    unsafe { o.initialise_data(agent, data, heap_byte_length, 0.into(), heap_array_length) };

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
    new_typed_array: Object<'a>,
    length: Option<i64>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, AnyTypedArray<'a>> {
    // 2. Let taRecord be ? ValidateTypedArray(newTypedArray, seq-cst).
    validate_typed_array_macro!(
        agent,
        new_typed_array.into_value(),
        ta_record,
        Ordering::SeqCst,
        gc,
        {
            let (o, cached_buffer_byte_length) = ta_record;
            // 3. If the number of elements in argumentList is 1 and argumentList[0] is a Number, then
            if let Some(first_arg) = length {
                // a. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
                if o.is_typed_array_out_of_bounds(agent, cached_buffer_byte_length) {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "TypedArray out of bounds",
                        gc,
                    ));
                }
                // b. Let length be TypedArrayLength(taRecord).
                let len = o.typed_array_length(agent, cached_buffer_byte_length) as i64;
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
            Ok(o.into())
        }
    )
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
) -> JsResult<'a, AnyTypedArray<'a>> {
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

#[inline(always)]
fn intrinsic_default_constructor<T: Viewable>() -> ProtoIntrinsics {
    {
        if TypeId::of::<T>() == TypeId::of::<i8>() {
            ProtoIntrinsics::Int8Array
        } else if TypeId::of::<T>() == TypeId::of::<u8>() {
            ProtoIntrinsics::Uint8Array
        } else if TypeId::of::<T>() == TypeId::of::<U8Clamped>() {
            ProtoIntrinsics::Uint8ClampedArray
        } else if TypeId::of::<T>() == TypeId::of::<i16>() {
            ProtoIntrinsics::Int16Array
        } else if TypeId::of::<T>() == TypeId::of::<u16>() {
            ProtoIntrinsics::Uint16Array
        } else if TypeId::of::<T>() == TypeId::of::<i32>() {
            ProtoIntrinsics::Int32Array
        } else if TypeId::of::<T>() == TypeId::of::<u32>() {
            ProtoIntrinsics::Uint32Array
        } else if TypeId::of::<T>() == TypeId::of::<i64>() {
            ProtoIntrinsics::BigInt64Array
        } else if TypeId::of::<T>() == TypeId::of::<u64>() {
            ProtoIntrinsics::BigUint64Array
        } else if TypeId::of::<T>() == TypeId::of::<f32>() {
            ProtoIntrinsics::Float32Array
        } else if TypeId::of::<T>() == TypeId::of::<f64>() {
            ProtoIntrinsics::Float64Array
        } else {
            #[cfg(feature = "proposal-float16array")]
            if TypeId::of::<T>() == TypeId::of::<f16>() {
                return ProtoIntrinsics::Float16Array;
            }
            unreachable!()
        }
    }
}

fn has_matching_content_type<T: Viewable>(result: AnyTypedArray) -> bool {
    let is_bigint = T::IS_BIGINT;
    let result_is_bigint = result.is_bigint();
    is_bigint == result_is_bigint
}

/// ### [23.2.4.1 TypedArraySpeciesCreate ( exemplar, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#typedarray-species-create)
pub(crate) fn try_typed_array_species_create_with_length<'gc>(
    agent: &mut Agent,
    exemplar: AnyTypedArray<'gc>,
    length: usize,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, DataBlock> {
    // 1. Let defaultConstructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let default_constructor = exemplar.intrinsic_default_constructor();
    let element_size = exemplar.typed_array_element_size();
    // 2. Let constructor be ? SpeciesConstructor(exemplar, defaultConstructor).
    let constructor =
        try_species_constructor(agent, exemplar.into_object(), default_constructor, gc)?;
    if constructor.is_some() {
        // We'd have to perform an actual Construct call; we cannot do that so
        // this is the end of the road.
        return TryError::GcError.into();
    }
    let Some(byte_length) = length.checked_mul(element_size) else {
        // We could actually throw an error here but this is really rare.
        return TryError::GcError.into();
    };
    // 3. Let result be ? TypedArrayCreateFromConstructor(constructor, argumentList).
    // 4. Assert: result has [[TypedArrayName]] and [[ContentType]] internal slots.
    // 5. If result.[[ContentType]] is not exemplar.[[ContentType]], throw a TypeError exception.
    // 6. Return result.
    // Note: we don't set the type ahead of time.
    js_result_into_try(create_byte_data_block(agent, byte_length as u64, gc))
}

/// ### [23.2.4.1 TypedArraySpeciesCreate ( exemplar, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#typedarray-species-create)
pub(crate) fn typed_array_species_create_with_length<'a, T: Viewable>(
    agent: &mut Agent,
    exemplar: Object,
    length: i64,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, AnyTypedArray<'a>> {
    // 1. Let defaultConstructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let default_constructor = intrinsic_default_constructor::<T>();
    // 2. Let constructor be ? SpeciesConstructor(exemplar, defaultConstructor).
    let constructor = species_constructor(agent, exemplar, default_constructor, gc.reborrow())
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

/// ### [23.2.4.1 TypedArraySpeciesCreate ( exemplar, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#typedarray-species-create)
pub(crate) fn typed_array_species_create_with_buffer<'a, T: Viewable>(
    agent: &mut Agent,
    exemplar: TypedArray,
    array_buffer: ArrayBuffer,
    byte_offset: i64,
    length: Option<i64>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, TypedArray<'a>> {
    // 1. Let defaultConstructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let default_constructor = intrinsic_default_constructor::<T>();
    // 2. Let constructor be ? SpeciesConstructor(exemplar, defaultConstructor).
    let constructor = species_constructor(
        agent,
        exemplar.into_object(),
        default_constructor,
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

/// ### [23.2.3.26.2 SetTypedArrayFromArrayLike ( target, targetOffset, source )](https://tc39.es/ecma262/multipage/indexed-collections.html#sec-settypedarrayfromarraylike)
/// The abstract operation SetTypedArrayFromArrayLike takes arguments target
/// (a TypedArray), targetOffset (a non-negative integer or +∞), and source
/// (an ECMAScript language value, but not a TypedArray) and returns either
/// a normal completion containing unused or a throw completion. It sets
/// multiple values in target, starting at index targetOffset, reading the
/// values from source.
pub(crate) fn set_typed_array_from_array_like<'a, T: Viewable>(
    agent: &mut Agent,
    target: GenericTypedArray<'a, T>,
    target_offset: IntegerOrInfinity,
    source: Scoped<Value>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let target = target.bind(gc.nogc());
    // 1. Let targetRecord be MakeTypedArrayWithBufferWitnessRecord(target, seq-cst).
    let (target, cached_buffer_byte_length) =
        make_typed_array_with_buffer_witness_record_specialised(agent, target, Ordering::SeqCst);
    // 2. If IsTypedArrayOutOfBounds(targetRecord) is true, throw a TypeError exception.
    if is_typed_array_out_of_bounds_specialised(agent, target, cached_buffer_byte_length) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc.into_nogc(),
        ));
    };
    // 3. Let targetLength be TypedArrayLength(targetRecord).
    let target_length =
        typed_array_length_specialised::<T>(agent, target, cached_buffer_byte_length) as u64;
    // 4. Let src be ? ToObject(source).
    let src = to_object(agent, source.get(agent), gc.nogc())
        .unbind()?
        .bind(gc.nogc());
    // SAFETY: source is not shared.
    let source = unsafe { source.replace_self(agent, src.unbind()) };
    let target = target.scope(agent, gc.nogc());
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
        typed_array_set_element(
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
