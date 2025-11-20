// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::hint::assert_unchecked;

use ecmascript_atomics::Ordering;

#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::{GenericSharedTypedArray, data::SharedTypedArrayRecord};
use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                construct, get, length_of_array_like, set, species_constructor,
                try_species_constructor,
            },
            type_conversion::{to_index, try_to_index},
        },
        builtins::{
            ArgumentsList, ArrayBuffer, ArrayBufferHeapData,
            array_buffer::{
                AnyArrayBuffer, get_value_from_buffer, is_fixed_length_array_buffer,
                set_value_in_buffer,
            },
            indexed_collections::typed_array_objects::typed_array_intrinsic_object::require_internal_slot_typed_array,
            ordinary::get_prototype_from_constructor,
            typed_array::{
                AnyTypedArray, GenericTypedArray, TypedArray, VoidArray, data::TypedArrayRecord,
            },
        },
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, TryError, TryResult, js_result_into_try, try_result_into_js},
        },
        types::{
            DataBlock, Function, InternalSlots, IntoObject, IntoValue, Number, Numeric, Object,
            PropertyKey, Value, Viewable, create_byte_data_block,
        },
    },
    engine::{
        Scoped, ScopedCollection,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::Scopable,
    },
    heap::CreateHeapData,
};

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

#[cfg(feature = "shared-array-buffer")]
pub(crate) fn shared_typed_array_create<'a, T: Viewable>(
    agent: &mut Agent,
    prototype: Option<Object<'a>>,
) -> GenericSharedTypedArray<'a, T> {
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
    let a = SharedTypedArrayRecord::default();

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
    /// Return `true` if this TypedArray is backed by a SharedArrayBuffer.
    fn is_shared(self) -> bool;

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
        let element_size = self.typed_array_element_size();
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
        let element_size = self.typed_array_element_size();

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
            let element_size = self.typed_array_element_size();
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
        check_not_void_array::<Self::ElementType>();
        // 1. If IsValidIntegerIndex(O, index) is false, return undefined.
        let index = self.is_valid_integer_index(agent, index)?;
        // 2. Let offset be O.[[ByteOffset]].
        let offset = self.byte_offset(agent);
        // 3. Let elementSize be TypedArrayElementSize(O).
        let element_size = self.typed_array_element_size();
        // 4. Let byteIndexInBuffer be (ℝ(index) × elementSize) + offset.
        let byte_index_in_buffer = (index * element_size) + offset;
        // 5. Let elementType be TypedArrayElementType(O).
        // 6. Return GetValueFromBuffer(O.[[ViewedArrayBuffer]], byteIndexInBuffer, elementType, true, unordered).
        Some(get_value_from_buffer::<Self::ElementType>(
            agent,
            self.viewed_array_buffer(agent),
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
        check_not_void_array::<Self::ElementType>();
        // 3. If IsValidIntegerIndex(O, index) is true, then
        if let Some(index) = self.is_valid_integer_index(agent, index) {
            // a. Let offset be O.[[ByteOffset]].
            let offset = self.byte_offset(agent);
            // b. Let elementSize be TypedArrayElementSize(O).
            let element_size = self.typed_array_element_size();
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

    fn set_into_data_block(
        self,
        agent: &Agent,
        target: &mut DataBlock,
        start_index: usize,
        count: usize,
    );

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
        source_offset: usize,
        length: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, ()>;

    fn slice(self, agent: &mut Agent, source: AnyTypedArray, source_offset: usize, length: usize);

    fn sort_with_comparator<'gc>(
        self,
        agent: &mut Agent,
        len: usize,
        comparator: Scoped<Function>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, ()>;

    fn sort(self, agent: &mut Agent, len: usize);

    fn typed_array_create_same_type_and_copy_data<'gc>(
        self,
        agent: &mut Agent,
        len: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, TypedArray<'gc>>;
}

fn check_not_void_array<T: Viewable>() {
    if core::any::TypeId::of::<T>() == core::any::TypeId::of::<()>() {
        panic!("Cannot call method on VoidArray");
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

/// ### [23.2.5.1.3 InitializeTypedArrayFromArrayBuffer ( O, buffer, byteOffset, length )](https://tc39.es/ecma262/#sec-initializetypedarrayfromarraybuffer)
///
/// The abstract operation InitializeTypedArrayFromArrayBuffer takes arguments
/// O (a TypedArray), buffer (an ArrayBuffer or a SharedArrayBuffer),
/// byteOffset (an ECMAScript language value), and length (an ECMAScript
/// language value) and returns either a normal completion containing unused or
/// a throw completion.
pub(crate) fn initialize_typed_array_from_array_buffer<'gc, T: Viewable>(
    agent: &mut Agent,
    o_proto: Option<Object>,
    buffer: AnyArrayBuffer,
    byte_offset: Option<Value>,
    length: Option<Value>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, AnyTypedArray<'gc>> {
    let mut o_proto = o_proto.bind(gc.nogc());
    let mut buffer = buffer.bind(gc.nogc());
    let byte_offset = byte_offset.bind(gc.nogc());
    let mut length = length.bind(gc.nogc());

    // 1. Let elementSize be TypedArrayElementSize(O).
    let element_size = size_of::<T>();

    // 2. Let offset be ? ToIndex(byteOffset).
    let offset = if let Some(byte_offset) = byte_offset {
        if let Some(offset) = try_result_into_js(try_to_index(agent, byte_offset, gc.nogc()))
            .unbind()?
            .bind(gc.nogc())
        {
            offset
        } else {
            let nogc = gc.nogc();
            let o = o_proto.map(|p| p.scope(agent, nogc));
            let b = buffer.scope(agent, nogc);
            let l = length.map(|l| l.scope(agent, nogc));
            let offset = to_index(agent, byte_offset.unbind(), gc.reborrow()).unbind()? as u64;
            unsafe {
                let nogc = gc.nogc();
                length = l.map(|l| l.take(agent)).bind(nogc);
                buffer = b.take(agent).bind(nogc);
                o_proto = o.map(|p| p.take(agent)).bind(nogc);
            }
            offset
        }
    } else {
        0
    };

    // 3. If offset modulo elementSize ≠ 0, throw a RangeError exception.
    if !offset.is_multiple_of(element_size as u64) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "offset is not a multiple of the element size",
            gc.into_nogc(),
        ));
    }

    // 4. Let bufferIsFixedLength be IsFixedLengthArrayBuffer(buffer).
    let buffer_is_fixed_length = is_fixed_length_array_buffer(agent, buffer);

    // 5. If length is not undefined, then
    // a. Let newLength be ? ToIndex(length).
    let new_length = if let Some(length) = length {
        // SAFETY: caller should have already mapped undefined to None.
        unsafe { assert_unchecked(!length.is_undefined()) };
        if let Some(length) = try_result_into_js(try_to_index(agent, length, gc.nogc()))
            .unbind()?
            .bind(gc.nogc())
        {
            Some(length)
        } else {
            let nogc = gc.nogc();
            let o = o_proto.map(|p| p.scope(agent, nogc));
            let b = buffer.scope(agent, nogc);
            let offset = to_index(agent, length.unbind(), gc.reborrow()).unbind()? as u64;
            unsafe {
                let nogc = gc.nogc();
                buffer = b.take(agent).bind(nogc);
                o_proto = o.map(|p| p.take(agent).bind(nogc));
            }
            Some(offset)
        }
    } else {
        None
    };

    let o_proto = o_proto.unbind();
    let buffer = buffer.unbind();
    let gc = gc.into_nogc();
    let o_proto = o_proto.bind(gc);
    let buffer = buffer.bind(gc);

    // 6. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
    if buffer.is_detached(agent) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "attempting to access detached ArrayBuffer",
            gc.into_nogc(),
        ));
    }

    // 7. Let bufferByteLength be ArrayBufferByteLength(buffer, seq-cst).
    let buffer_byte_length = buffer.byte_length(agent, Ordering::SeqCst);

    // 8. If length is undefined and bufferIsFixedLength is false, then
    if new_length.is_none() && !buffer_is_fixed_length {
        // a. If offset > bufferByteLength, throw a RangeError exception.
        if offset > buffer_byte_length as u64 {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "offset is outside the bounds of the buffer",
                gc.into_nogc(),
            ));
        }

        // b. Set O.[[ByteLength]] to auto.
        // c. Set O.[[ArrayLength]] to auto.
        // 10. Set O.[[ViewedArrayBuffer]] to buffer.
        // 11. Set O.[[ByteOffset]] to offset.
        match buffer {
            AnyArrayBuffer::ArrayBuffer(buffer) => {
                let o = typed_array_create::<T>(agent, o_proto);
                // SAFETY: We are initialising O.
                unsafe { o.initialise_data(agent, buffer, offset as usize, None) };
                Ok(o.into())
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(buffer) => {
                let o = shared_typed_array_create::<T>(agent, o_proto);
                // SAFETY: We are initialising O.
                unsafe { o.initialise_data(agent, buffer, offset as usize, None) };
                Ok(o.into())
            }
        }
    } else {
        // 9. Else,
        let new_byte_length = if let Some(new_length) = new_length {
            // b. Else,
            // i. Let newByteLength be newLength × elementSize.
            let new_byte_length = new_length.saturating_mul(element_size as u64);
            // ii. If offset + newByteLength > bufferByteLength, throw a RangeError exception.
            if offset.saturating_add(new_byte_length) > buffer_byte_length as u64 {
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
        if let Some(new_byte_length) = (buffer_byte_length as u64).checked_sub(offset) {
            new_byte_length
        } else {
            // iii. If newByteLength < 0, throw a RangeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "new byte length is negative",
                gc.into_nogc(),
            ));
        };

        let new_byte_length = new_byte_length as usize;
        let array_length = new_byte_length / element_size;
        // c. Set O.[[ByteLength]] to newByteLength.
        // d. Set O.[[ArrayLength]] to newByteLength / elementSize.
        // 10. Set O.[[ViewedArrayBuffer]] to buffer.
        // 11. Set O.[[ByteOffset]] to offset.
        match buffer {
            AnyArrayBuffer::ArrayBuffer(buffer) => {
                let o = typed_array_create::<T>(agent, o_proto);
                // SAFETY: We are initialising O.
                unsafe {
                    o.initialise_data(
                        agent,
                        buffer,
                        offset as usize,
                        Some((new_byte_length, array_length)),
                    )
                };
                Ok(o.into())
            }
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(buffer) => {
                let o = shared_typed_array_create::<T>(agent, o_proto);
                // SAFETY: We are initialising O.
                unsafe {
                    o.initialise_data(
                        agent,
                        buffer,
                        offset as usize,
                        Some((new_byte_length, array_length)),
                    )
                };
                Ok(o.into())
            }
        }
    }
    // 12. Return unused.
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
    o: GenericTypedArray<T>,
    array_like: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let o = o.scope(agent, gc.nogc());
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

    // 5. Set O.[[ViewedArrayBuffer]] to data.
    // 6. Set O.[[ByteLength]] to byteLength.
    // 7. Set O.[[ByteOffset]] to 0.
    // 8. Set O.[[ArrayLength]] to length.
    // SAFETY: We're initialising O.
    unsafe { o.initialise_data(agent, data, 0, Some((byte_length, length))) };

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
    let ta_record =
        validate_typed_array(agent, new_typed_array.into_value(), Ordering::SeqCst, gc)?;
    // 3. If the number of elements in argumentList is 1 and argumentList[0] is a Number, then
    if let Some(length) = length {
        // a. If IsTypedArrayOutOfBounds(taRecord) is true, throw a TypeError exception.
        if ta_record.is_typed_array_out_of_bounds(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "TypedArray out of bounds",
                gc,
            ));
        }
        // b. Let length be TypedArrayLength(taRecord).
        let len = ta_record.typed_array_length(agent);
        // c. If length < ℝ(argumentList[0]), throw a TypeError exception.
        if length > 0
            && usize::try_from(length)
                .ok()
                .is_none_or(|length| len < length)
        {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "TypedArray out of bounds",
                gc,
            ));
        };
    }
    // 4. Return newTypedArray.
    Ok(ta_record.object)
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
    let arg0 = Number::from_i64(agent, length, gc.nogc()).into_value();
    // 1. Let newTypedArray be ? Construct(constructor, argumentList).
    let new_typed_array = construct(
        agent,
        constructor.unbind(),
        Some(ArgumentsList::from_mut_value(&mut arg0.unbind())),
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
    buffer: AnyArrayBuffer,
    byte_offset: usize,
    length: Option<usize>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, AnyTypedArray<'a>> {
    let constructor = constructor.bind(gc.nogc());
    let buffer = buffer.bind(gc.nogc());
    // 1. Let newTypedArray be ? Construct(constructor, argumentList).
    let new_typed_array = {
        let args: &mut [Value] = if let Some(length) = length {
            &mut [
                buffer.into_value().unbind(),
                Number::from_usize(agent, byte_offset, gc.nogc())
                    .into_value()
                    .unbind(),
                Number::from_usize(agent, length, gc.nogc())
                    .into_value()
                    .unbind(),
            ]
        } else {
            &mut [
                buffer.into_value().unbind(),
                Number::from_usize(agent, byte_offset, gc.nogc())
                    .into_value()
                    .unbind(),
            ]
        };

        construct(
            agent,
            constructor.unbind(),
            Some(ArgumentsList::from_mut_slice(args)),
            None,
            gc.reborrow(),
        )
    }
    .unbind()?
    .bind(gc.nogc());
    let length = length.map(|l| i64::try_from(l).unwrap());
    typed_array_create_from_constructor_internal(
        agent,
        new_typed_array.unbind(),
        length,
        gc.into_nogc(),
    )
}

pub(crate) fn typed_array_create_from_data_block<'a>(
    agent: &mut Agent,
    exemplar: impl Into<AnyTypedArray<'a>>,
    data_block: DataBlock,
) -> VoidArray<'a> {
    let exemplar = exemplar.into();
    let element_size = exemplar.typed_array_element_size();
    let byte_length = data_block.len();
    let array_length = byte_length / element_size;
    let ab = agent
        .heap
        .create(ArrayBufferHeapData::new_fixed_length(data_block));
    let result: VoidArray = agent.heap.create(TypedArrayRecord::default());
    // SAFETY: Initialising new TypedArrayRecord.
    unsafe { result.initialise_data(agent, ab, 0, Some((byte_length, array_length))) };
    // 5. Return result.
    result
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
    let byte_length = (length as u64).saturating_mul(element_size as u64);
    // 3. Let result be ? TypedArrayCreateFromConstructor(constructor, argumentList).
    // 4. Assert: result has [[TypedArrayName]] and [[ContentType]] internal slots.
    // 5. If result.[[ContentType]] is not exemplar.[[ContentType]], throw a TypeError exception.
    // 6. Return result.
    // Note: we don't set the type ahead of time.
    js_result_into_try(create_byte_data_block(agent, byte_length, gc))
}

/// ### [23.2.4.1 TypedArraySpeciesCreate ( exemplar, argumentList )](https://tc39.es/ecma262/multipage/indexed-collections.html#typedarray-species-create)
pub(crate) fn typed_array_species_create_with_length<'gc>(
    agent: &mut Agent,
    exemplar: AnyTypedArray,
    length: usize,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, AnyTypedArray<'gc>> {
    let exemplar = exemplar.bind(gc.nogc());
    // 1. Let defaultConstructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let default_constructor = exemplar.intrinsic_default_constructor();
    let is_bigint = exemplar.is_bigint();
    // 2. Let constructor be ? SpeciesConstructor(exemplar, defaultConstructor).
    let constructor = species_constructor(
        agent,
        exemplar.into_object().unbind(),
        default_constructor,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    let length = i64::try_from(length).unwrap();
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
    if is_bigint != result.is_bigint() {
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
pub(crate) fn typed_array_species_create_with_buffer<'a>(
    agent: &mut Agent,
    exemplar: AnyTypedArray,
    buffer: AnyArrayBuffer,
    byte_offset: usize,
    length: Option<usize>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, AnyTypedArray<'a>> {
    let exemplar = exemplar.bind(gc.nogc());
    let buffer = buffer.scope(agent, gc.nogc());
    // 1. Let defaultConstructor be the intrinsic object associated with the constructor name exemplar.[[TypedArrayName]] in Table 73.
    let default_constructor = exemplar.intrinsic_default_constructor();
    let is_bigint = exemplar.is_bigint();
    // 2. Let constructor be ? SpeciesConstructor(exemplar, defaultConstructor).
    let constructor = species_constructor(
        agent,
        exemplar.into_object().unbind(),
        default_constructor,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. Let result be ? TypedArrayCreateFromConstructor(constructor, argumentList).
    let result = typed_array_create_from_constructor_with_buffer(
        agent,
        constructor.unbind(),
        // SAFETY: not shared.
        unsafe { buffer.take(agent) },
        byte_offset,
        length,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 4. Assert: result has [[TypedArrayName]] and [[ContentType]] internal slots.
    // 5. If result.[[ContentType]] is not exemplar.[[ContentType]], throw a TypeError exception.
    if is_bigint != result.is_bigint() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "can't convert BigInt to number",
            gc.into_nogc(),
        ));
    }
    // 6. Return result.
    Ok(result.unbind())
}
