// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{AnyArrayBuffer, ArrayBuffer, InternalBuffer};
use crate::{
    Heap,
    ecmascript::{
        abstract_operations::{operations_on_objects::get, type_conversion::to_index},
        builtins::ordinary::ordinary_create_from_constructor,
        execution::{Agent, JsResult, ProtoIntrinsics, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, Number, Numeric, Object, SharedDataBlock, Value,
            Viewable, copy_data_block_bytes, create_byte_data_block,
        },
    },
    engine::context::{Bindable, GcScope, NoGcScope},
};

// TODO: Implement the contents of the `DetachKey` struct?
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct DetachKey {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub(crate) enum Ordering {
    Unordered = core::sync::atomic::Ordering::Relaxed as u8,
    SeqCst = core::sync::atomic::Ordering::SeqCst as u8,
    #[expect(dead_code)]
    Init,
}

impl From<Ordering> for ecmascript_atomics::Ordering {
    fn from(value: Ordering) -> Self {
        match value {
            Ordering::Unordered => Self::Unordered,
            Ordering::SeqCst => Self::SeqCst,
            // This shouldn't ever actually happen.
            Ordering::Init => Self::SeqCst,
        }
    }
}

/// ### [25.1.3.1 AllocateArrayBuffer ( constructor, byteLength \[ , maxByteLength \] )](https://tc39.es/ecma262/#sec-allocatearraybuffer)
///
/// The abstract operation AllocateArrayBuffer takes arguments *constructor*
/// (a constructor) and *byteLength* (a non-negative integer) and optional
/// argument *maxByteLength* (a non-negative integer or EMPTY) and returns
/// either a normal completion containing an ArrayBuffer or a throw
/// completion. It is used to create an ArrayBuffer.
pub(crate) fn allocate_array_buffer<'a>(
    agent: &mut Agent,
    constructor: Function,
    byte_length: u64,
    max_byte_length: Option<u64>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ArrayBuffer<'a>> {
    let constructor = constructor.bind(gc.nogc());
    // 1. Let slots be « [[ArrayBufferData]], [[ArrayBufferByteLength]], [[ArrayBufferDetachKey]] ».
    // 2. If maxByteLength is present and maxByteLength is not EMPTY, let allocatingResizableBuffer be true; otherwise let allocatingResizableBuffer be false.
    // 3. If allocatingResizableBuffer is true, then
    if let Some(max_byte_length) = max_byte_length {
        // a. If byteLength > maxByteLength, throw a RangeError exception.
        if byte_length > max_byte_length {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Byte length is over maximumm byte length",
                gc.into_nogc(),
            ));
        }
        // b. Append [[ArrayBufferMaxByteLength]] to slots.
    }
    // 4. Let obj be ? OrdinaryCreateFromConstructor(constructor, "%ArrayBuffer.prototype%", slots).
    let Object::ArrayBuffer(obj) = ordinary_create_from_constructor(
        agent,
        constructor.unbind(),
        ProtoIntrinsics::ArrayBuffer,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc()) else {
        unreachable!()
    };
    let obj = obj.unbind();
    let gc = gc.into_nogc();
    let obj = obj.bind(gc);
    // 5. Let block be ? CreateByteDataBlock(byteLength).
    // 8. If allocatingResizableBuffer is true, then
    let buffer = if let Some(max_byte_length) = max_byte_length {
        // a. If it is not possible to create a Data Block block consisting of
        //    maxByteLength bytes, throw a RangeError exception.
        // b. NOTE: Resizable ArrayBuffers are designed to be implementable
        //    with in-place growth. Implementations may throw if, for example,
        //    virtual memory cannot be reserved up front.
        // c. Set obj.[[ArrayBufferMaxByteLength]] to maxByteLength.
        let mut block = create_byte_data_block(agent, max_byte_length, gc)?;
        block.realloc(byte_length as usize);
        InternalBuffer::resizable(block, max_byte_length as usize)
    } else {
        InternalBuffer::fixed_length(create_byte_data_block(agent, byte_length, gc)?)
    };
    // 6. Set obj.[[ArrayBufferData]] to block.
    // 7. Set obj.[[ArrayBufferByteLength]] to byteLength.
    agent[obj].buffer = buffer;
    // 9. Return obj.
    Ok(obj)
}

/// ### [25.1.3.2 ArrayBufferByteLength ( arrayBuffer, order )](https://tc39.es/ecma262/#sec-arraybufferbytelength)
///
/// The abstract operation ArrayBufferByteLength takes arguments arrayBuffer
/// (an ArrayBuffer or SharedArrayBuffer) and order (SEQ-CST or UNORDERED)
/// and returns a non-negative integer.
pub(crate) fn array_buffer_byte_length(agent: &Agent, array_buffer: ArrayBuffer) -> usize {
    let array_buffer = &agent[array_buffer];
    // 1. If IsSharedArrayBuffer(arrayBuffer) is true and arrayBuffer has an [[ArrayBufferByteLengthData]] internal slot, then
    // a. Let bufferByteLengthBlock be arrayBuffer.[[ArrayBufferByteLengthData]].
    // b. Let rawLength be GetRawBytesFromSharedBlock(bufferByteLengthBlock, 0, BIGUINT64, true, order).
    // c. Let isLittleEndian be the value of the [[LittleEndian]] field of the surrounding agent's Agent Record.
    // d. Return ℝ(RawBytesToNumeric(BIGUINT64, rawLength, isLittleEndian)).
    // 2. Assert: IsDetachedBuffer(arrayBuffer) is false.
    debug_assert!(!array_buffer.is_detached());
    // 3. Return arrayBuffer.[[ArrayBufferByteLength]].
    array_buffer.byte_length()
}

/// ### [25.1.3.3 IsDetachedBuffer ( arrayBuffer )](https://tc39.es/ecma262/#sec-isdetachedbuffer)
///
/// The abstract operation IsDetachedBuffer takes argument *arrayBuffer* (an
/// ArrayBuffer or a SharedArrayBuffer) and returns a Boolean.
#[inline]
pub(crate) fn is_detached_buffer(agent: &Agent, array_buffer: ArrayBuffer) -> bool {
    // 1. If arrayBuffer.[[ArrayBufferData]] is null, return true.
    // 2. Return false.
    agent[array_buffer].is_detached()
}

/// ### [25.1.3.4 DetachArrayBuffer ( arrayBuffer \[ , key \] )](https://tc39.es/ecma262/#sec-detacharraybuffer)
///
/// The abstract operation DetachArrayBuffer takes argument *arrayBuffer* (an
/// ArrayBuffer) and optional argument *key* (anything) and returns either a
/// normal completion containing UNUSED or a throw completion.
pub(crate) fn detach_array_buffer<'a>(
    agent: &mut Agent,
    array_buffer: ArrayBuffer,
    key: Option<DetachKey>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Assert: IsSharedArrayBuffer(arrayBuffer) is false.
    // TODO: SharedArrayBuffer that we can even take here.

    // 2. If key is not present, set key to undefined.
    // 3. If arrayBuffer.[[ArrayBufferDetachKey]] is not key, throw a TypeError exception.
    if array_buffer.get_detach_key(agent) != key {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Mismatching array buffer detach keys",
            gc,
        ));
    }

    // 4. Set arrayBuffer.[[ArrayBufferData]] to null.
    // 5. Set arrayBuffer.[[ArrayBufferByteLength]] to 0.
    agent[array_buffer].buffer.detach();
    // 6. Return UNUSED.
    Ok(())
}

/// ### [25.1.3.5 CloneArrayBuffer ( srcBuffer, srcByteOffset, srcLength )](https://tc39.es/ecma262/#sec-clonearraybuffer)
///
/// The abstract operation CloneArrayBuffer takes arguments srcBuffer (an
/// ArrayBuffer or a SharedArrayBuffer), srcByteOffset (a non-negative
/// integer), and srcLength (a non-negative integer) and returns either a
/// normal completion containing an ArrayBuffer or a throw completion. It
/// creates a new ArrayBuffer whose data is a copy of srcBuffer's data over the
/// range starting at srcByteOffset and continuing for srcLength bytes.
pub(crate) fn clone_array_buffer<'a>(
    agent: &mut Agent,
    src_buffer: ArrayBuffer<'a>,
    src_byte_offset: usize,
    src_length: usize,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ArrayBuffer<'a>> {
    // 1. Assert: IsDetachedBuffer(srcBuffer) is false.
    debug_assert!(!src_buffer.is_detached(agent));
    // 2. Let targetBuffer be ? AllocateArrayBuffer(%ArrayBuffer%, srcLength).
    let target_buffer = ArrayBuffer::new(agent, src_length, gc)?;

    let Heap { array_buffers, .. } = &mut agent.heap;
    let (target_buffer_data, array_buffers) = array_buffers.split_last_mut().unwrap();
    let target_buffer_data = target_buffer_data.as_mut().unwrap();
    let src_buffer = array_buffers
        .get(src_buffer.get_index())
        .unwrap()
        .as_ref()
        .unwrap();
    // 3. Let srcBlock be srcBuffer.[[ArrayBufferData]].
    let src_block = src_buffer.get_data_block();
    // 4. Let targetBlock be targetBuffer.[[ArrayBufferData]].
    let target_block = target_buffer_data.get_data_block_mut();
    // 5. Perform CopyDataBlockBytes(targetBlock, 0, srcBlock, srcByteOffset, srcLength).
    copy_data_block_bytes(target_block, 0, src_block, src_byte_offset, src_length);
    // 6. Return targetBuffer.
    Ok(target_buffer)
}

/// ### [25.1.3.6 GetArrayBufferMaxByteLengthOption ( options )](https://tc39.es/ecma262/#sec-getarraybuffermaxbytelengthoption)
///
/// The abstract operation GetArrayBufferMaxByteLengthOption takes argument
/// options (an ECMAScript language value) and returns either a normal
/// completion containing either a non-negative integer or EMPTY, or a throw
/// completion.
pub(crate) fn get_array_buffer_max_byte_length_option<'a>(
    agent: &mut Agent,
    options: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<u64>> {
    let options = options.bind(gc.nogc());
    // 1. If options is not an Object, return EMPTY.
    let options = if let Ok(options) = Object::try_from(options) {
        options
    } else {
        return Ok(None);
    };
    // 2. Let maxByteLength be ? Get(options, "maxByteLength").
    let max_byte_length = get(
        agent,
        options.unbind(),
        BUILTIN_STRING_MEMORY.maxByteLength.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. If maxByteLength is undefined, return EMPTY.
    if max_byte_length.is_undefined() {
        return Ok(None);
    }
    // 4. Return ? ToIndex(maxByteLength).
    Ok(Some(to_index(agent, max_byte_length.unbind(), gc)? as u64))
}

/// ### [25.1.3.8 IsFixedLengthArrayBuffer ( arrayBuffer )](https://tc39.es/ecma262/#sec-isfixedlengtharraybuffer)
///
/// The abstract operation IsFixedLengthArrayBuffer takes argument
/// arrayBuffer (an ArrayBuffer or a SharedArrayBuffer) and returns a
/// Boolean.
#[inline(always)]
pub(crate) fn is_fixed_length_array_buffer(agent: &Agent, array_buffer: AnyArrayBuffer) -> bool {
    // 1. If arrayBuffer has an [[ArrayBufferMaxByteLength]] internal slot, return false.
    // 2. Return true.
    !array_buffer.is_resizable(agent)
}

/// ### [25.1.3.13 RawBytesToNumeric ( type, rawBytes, isLittleEndian )](https://tc39.es/ecma262/#sec-rawbytestonumeric)
///
/// The abstract operation RawBytesToNumeric takes arguments type (a
/// TypedArray element type), rawBytes (a List of byte values), and
/// isLittleEndian (a Boolean) and returns a Number or a BigInt.
#[inline(always)]
pub(crate) fn raw_bytes_to_numeric<'a, T: Viewable>(
    agent: &mut Agent,
    raw_bytes: T,
    is_little_endian: bool,
    gc: NoGcScope<'a, '_>,
) -> Numeric<'a> {
    // 1. Let elementSize be the Element Size value specified in Table 71 for Element Type type.
    // 2. If isLittleEndian is false, reverse the order of the elements of rawBytes.
    // 3. If type is FLOAT32, then
    // a. Let value be the byte elements of rawBytes concatenated and interpreted as a little-endian bit string encoding of an IEEE 754-2019 binary32 value.
    // b. If value is an IEEE 754-2019 binary32 NaN value, return the NaN Number value.
    // c. Return the Number value that corresponds to value.
    // 4. If type is FLOAT64, then
    // a. Let value be the byte elements of rawBytes concatenated and interpreted as a little-endian bit string encoding of an IEEE 754-2019 binary64 value.
    // b. If value is an IEEE 754-2019 binary64 NaN value, return the NaN Number value.
    // c. Return the Number value that corresponds to value.
    // 5. If IsUnsignedElementType(type) is true, then
    // a. Let intValue be the byte elements of rawBytes concatenated and interpreted as a bit string encoding of an unsigned little-endian binary number.
    // 6. Else,
    // a. Let intValue be the byte elements of rawBytes concatenated and interpreted as a bit string encoding of a binary little-endian two's complement number of bit length elementSize × 8.
    // 7. If IsBigIntElementType(type) is true, return the BigInt value that corresponds to intValue.
    // 8. Otherwise, return the Number value that corresponds to intValue.
    if is_little_endian {
        raw_bytes.into_le_value(agent, gc)
    } else {
        raw_bytes.into_be_value(agent, gc)
    }
}

/// ### [25.1.3.14 GetRawBytesFromSharedBlock ( block, byteIndex, type, isTypedArray, order )](https://tc39.es/ecma262/#sec-getrawbytesfromsharedblock)
///
/// The abstract operation GetRawBytesFromSharedBlock takes arguments block
/// (a Shared Data Block), byteIndex (a non-negative integer), type (a
/// TypedArray element type), isTypedArray (a Boolean), and order (SEQ-CST
/// or UNORDERED) and returns a List of byte values.
#[inline(always)]
pub(crate) fn get_raw_bytes_from_shared_block<T: Viewable>(
    block: &SharedDataBlock,
    byte_index: usize,
    is_typed_array: bool,
    order: Ordering,
) -> T {
    // 1. Let elementSize be the Element Size value specified in Table 71 for Element Type type.
    // 2. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
    // 3. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
    // 4. If isTypedArray is true and IsNoTearConfiguration(type, order) is true, let noTear be true; otherwise let noTear be false.
    // 5. Let rawValue be a List of length elementSize whose elements are nondeterministically chosen byte values.
    // 6. NOTE: In implementations, rawValue is the result of a non-atomic or atomic read instruction on the underlying hardware. The nondeterminism is a semantic prescription of the memory model to describe observable behaviour of hardware with weak consistency.
    // 7. Let readEvent be ReadSharedMemory { [[Order]]: order, [[NoTear]]: noTear, [[Block]]: block, [[ByteIndex]]: byteIndex, [[ElementSize]]: elementSize }.
    // 8. Append readEvent to eventsRecord.[[EventList]].
    // 9. Append Chosen Value Record { [[Event]]: readEvent, [[ChosenValue]]: rawValue } to execution.[[ChosenValues]].
    // 10. Return rawValue.
    if is_typed_array {
        block.load::<T>(byte_index, order.into()).expect("Invalid GetRawBytesFromSharedBlock call: did not check index alignment and buffer length beforehand")
    } else {
        block.load_unaligned::<T>(byte_index).expect(
            "Invalid GetRawBytesFromSharedBlock call: did not check buffer length beforehand",
        )
    }
}

/// ### [25.1.3.15 GetValueFromBuffer ( arrayBuffer, byteIndex, type, isTypedArray, order \[ , isLittleEndian \] )](https://tc39.es/ecma262/#sec-getvaluefrombuffer)
///
/// The abstract operation GetValueFromBuffer takes arguments arrayBuffer
/// (an ArrayBuffer or SharedArrayBuffer), byteIndex (a non-negative
/// integer), type (a TypedArray element type), isTypedArray (a Boolean),
/// and order (SEQ-CST or UNORDERED) and optional argument isLittleEndian
/// (a Boolean) and returns a Number or a BigInt.
pub(crate) fn get_value_from_buffer<'a, T: Viewable>(
    agent: &mut Agent,
    array_buffer: AnyArrayBuffer,
    byte_index: usize,
    is_typed_array: bool,
    order: Ordering,
    is_little_endian: Option<bool>,
    gc: NoGcScope<'a, '_>,
) -> Numeric<'a> {
    // 1. Assert: IsDetachedBuffer(arrayBuffer) is false.
    debug_assert!(!array_buffer.is_detached(agent));
    // 2. Assert: There are sufficient bytes in arrayBuffer starting at byteIndex to represent a value of type.
    // 4. Let elementSize be the Element Size value specified in Table 71 for Element Type type.
    let raw_value = match array_buffer {
        // 5. If IsSharedArrayBuffer(arrayBuffer) is true, then
        AnyArrayBuffer::SharedArrayBuffer(sab) => {
            // 3. Let block be arrayBuffer.[[ArrayBufferData]].
            let block = sab.get_data_block(agent);
            // a. Assert: block is a Shared Data Block.
            // b. Let rawValue be GetRawBytesFromSharedBlock(block, byteIndex, type,
            //    isTypedArray, order).
            get_raw_bytes_from_shared_block::<T>(block, byte_index, is_typed_array, order)
        }
        // 6. Else,
        AnyArrayBuffer::ArrayBuffer(ab) => {
            // 3. Let block be arrayBuffer.[[ArrayBufferData]].
            let block = agent[ab].get_data_block();
            // a. Let rawValue be a List whose elements are bytes from block at indices
            //    in the interval from byteIndex (inclusive) to byteIndex + elementSize
            //    (exclusive).
            if is_typed_array {
                // SAFETY: Caller checked.
                unsafe { block.read_aligned(byte_index) }
            } else {
                // SAFETY: Caller checked.
                unsafe { block.read_unaligned(byte_index) }
            }
        }
    };
    // 7. Assert: The number of elements in rawValue is elementSize.
    // 8. If isLittleEndian is not present, set isLittleEndian to the value of
    //    the [[LittleEndian]] field of the surrounding agent's Agent Record.
    let is_little_endian = is_little_endian.unwrap_or({
        #[cfg(target_endian = "little")]
        {
            true
        }
        #[cfg(target_endian = "big")]
        {
            false
        }
    });

    // 9. Return RawBytesToNumeric(type, rawValue, isLittleEndian).
    raw_bytes_to_numeric::<T>(agent, raw_value, is_little_endian, gc)
}

/// ### [25.1.3.16 NumericToRawBytes ( type, value, isLittleEndian )](https://tc39.es/ecma262/#sec-numerictorawbytes)
///
/// The abstract operation NumericToRawBytes takes arguments type (a
/// TypedArray element type), value (a Number or a BigInt), and
/// isLittleEndian (a Boolean) and returns a List of byte values.
pub(crate) fn numeric_to_raw_bytes<T: Viewable>(
    agent: &mut Agent,
    value: Numeric,
    is_little_endian: bool,
) -> T {
    // 1. If type is FLOAT32, then
    // a. Let rawBytes be a List whose elements are the 4 bytes that are the result of converting value to IEEE 754-2019 binary32 format using roundTiesToEven mode. The bytes are arranged in little endian order. If value is NaN, rawBytes may be set to any implementation chosen IEEE 754-2019 binary32 format Not-a-Number encoding. An implementation must always choose the same encoding for each implementation distinguishable NaN value.
    // 2. Else if type is FLOAT64, then
    // a. Let rawBytes be a List whose elements are the 8 bytes that are the IEEE 754-2019 binary64 format encoding of value. The bytes are arranged in little endian order. If value is NaN, rawBytes may be set to any implementation chosen IEEE 754-2019 binary64 format Not-a-Number encoding. An implementation must always choose the same encoding for each implementation distinguishable NaN value.
    // 3. Else,
    // a. Let n be the Element Size value specified in Table 71 for Element Type type.
    // b. Let convOp be the abstract operation named in the Conversion Operation column in Table 71 for Element Type type.
    // c. Let intValue be ℝ(convOp(value)).
    // d. If intValue ≥ 0, then
    // i. Let rawBytes be a List whose elements are the n-byte binary encoding of intValue. The bytes are ordered in little endian order.
    // e. Else,
    // i. Let rawBytes be a List whose elements are the n-byte binary two's complement encoding of intValue. The bytes are ordered in little endian order.
    // 4. If isLittleEndian is false, reverse the order of the elements of rawBytes.
    // 5. Return rawBytes.
    if is_little_endian {
        T::from_le_value(agent, value)
    } else {
        T::from_be_value(agent, value)
    }
}

/// ### [25.1.3.17 SetValueInBuffer ( arrayBuffer, byteIndex, type, value, isTypedArray, order \[ , isLittleEndian \] )](https://tc39.es/ecma262/#sec-setvalueinbuffer)
///
/// The abstract operation SetValueInBuffer takes arguments arrayBuffer (an
/// ArrayBuffer or SharedArrayBuffer), byteIndex (a non-negative integer),
/// type (a TypedArray element type), value (a Number or a BigInt),
/// isTypedArray (a Boolean), and order (SEQ-CST, UNORDERED, or INIT) and
/// optional argument isLittleEndian (a Boolean) and returns UNUSED.
#[allow(clippy::too_many_arguments)]
pub(crate) fn set_value_in_buffer<T: Viewable>(
    agent: &mut Agent,
    array_buffer: AnyArrayBuffer,
    byte_index: usize,
    value: Numeric,
    is_typed_array: bool,
    order: Ordering,
    is_little_endian: Option<bool>,
) {
    // 1. Assert: IsDetachedBuffer(arrayBuffer) is false.
    debug_assert!(!array_buffer.is_detached(agent));
    // 2. Assert: There are sufficient bytes in arrayBuffer starting at byteIndex to represent a value of type.
    // 3. Assert: value is a BigInt if IsBigIntElementType(type) is true; otherwise, value is a Number.

    // 5. Let elementSize be the Element Size value specified in Table 71 for Element Type type.
    // 6. If isLittleEndian is not present, set isLittleEndian to the value of the [[LittleEndian]] field of the surrounding agent's Agent Record.
    let is_little_endian = is_little_endian.unwrap_or({
        #[cfg(target_endian = "little")]
        {
            true
        }
        #[cfg(target_endian = "big")]
        {
            false
        }
    });

    // 7. Let rawBytes be NumericToRawBytes(type, value, isLittleEndian).
    let raw_bytes = numeric_to_raw_bytes::<T>(agent, value, is_little_endian);
    match array_buffer {
        // 8. If IsSharedArrayBuffer(arrayBuffer) is true, then
        AnyArrayBuffer::SharedArrayBuffer(sab) => {
            // 4. Let block be arrayBuffer.[[ArrayBufferData]].
            let block = sab.get_data_block(agent);
            // a. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
            // b. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
            // c. If isTypedArray is true and IsNoTearConfiguration(type, order) is true, let noTear be true; otherwise let noTear be false.
            // d. Append WriteSharedMemory { [[Order]]: order, [[NoTear]]: noTear, [[Block]]: block, [[ByteIndex]]: byteIndex, [[ElementSize]]: elementSize, [[Payload]]: rawBytes } to eventsRecord.[[EventList]].
            if is_typed_array {
                block
                    .store(byte_index, raw_bytes, order.into())
                    .expect("Unaligned store or SharedDataBlock not large enough")
            } else {
                block
                    .store_unaligned(byte_index, raw_bytes)
                    .expect("SharedDataBlock not large enough")
            }
        }
        // 9. Else,
        AnyArrayBuffer::ArrayBuffer(ab) => {
            // 4. Let block be arrayBuffer.[[ArrayBufferData]].
            let block = agent[ab].get_data_block_mut();
            // a. Store the individual bytes of rawBytes into block, starting at block[byteIndex].
            block.set_offset_by_byte::<T>(byte_index, raw_bytes);
        }
    }

    // 10. Return UNUSED.
}

/// ### [25.1.3.18 GetModifySetValueInBuffer ( arrayBuffer, byteIndex, type, value, op )](https://tc39.es/ecma262/#sec-getmodifysetvalueinbuffer)
///
/// The abstract operation GetModifySetValueInBuffer takes arguments
/// arrayBuffer (an ArrayBuffer or a SharedArrayBuffer), byteIndex (a
/// non-negative integer), type (a TypedArray element type), value (a Number or
/// a BigInt), and op (a read-modify-write modification function) and returns a
/// Number or a BigInt.
#[expect(dead_code)]
pub(crate) fn get_modify_set_value_in_buffer(
    _array_buffer: ArrayBuffer,
    _byte_index: u32,
    _type: (),
    _value: Number,
    _op: (),
) {
    // 1. Assert: IsDetachedBuffer(arrayBuffer) is false.
    // 2. Assert: There are sufficient bytes in arrayBuffer starting at byteIndex to represent a value of type.
    // 3. Assert: value is a BigInt if IsBigIntElementType(type) is true; otherwise, value is a Number.
    // 4. Let block be arrayBuffer.[[ArrayBufferData]].
    // 5. Let elementSize be the Element Size value specified in Table 71 for Element Type type.
    // 6. Let isLittleEndian be the value of the [[LittleEndian]] field of the surrounding agent's Agent Record.
    // 7. Let rawBytes be NumericToRawBytes(type, value, isLittleEndian).
    // 8. If IsSharedArrayBuffer(arrayBuffer) is true, then
    // a. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
    // b. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
    // c. Let rawBytesRead be a List of length elementSize whose elements are nondeterministically chosen byte values.
    // d. NOTE: In implementations, rawBytesRead is the result of a load-link, of a load-exclusive, or of an operand of a read-modify-write instruction on the underlying hardware. The nondeterminism is a semantic prescription of the memory model to describe observable behaviour of hardware with weak consistency.
    // e. Let rmwEvent be ReadModifyWriteSharedMemory { [[Order]]: SEQ-CST, [[NoTear]]: true, [[Block]]: block, [[ByteIndex]]: byteIndex, [[ElementSize]]: elementSize, [[Payload]]: rawBytes, [[ModifyOp]]: op }.
    // f. Append rmwEvent to eventsRecord.[[EventList]].
    // g. Append Chosen Value Record { [[Event]]: rmwEvent, [[ChosenValue]]: rawBytesRead } to execution.[[ChosenValues]].
    // 9. Else,
    // a. Let rawBytesRead be a List of length elementSize whose elements are the sequence of elementSize bytes starting with block[byteIndex].
    // b. Let rawBytesModified be op(rawBytesRead, rawBytes).
    // c. Store the individual bytes of rawBytesModified into block, starting at block[byteIndex].
    // 10. Return RawBytesToNumeric(type, rawBytesRead, isLittleEndian).
}
