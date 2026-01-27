// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ExceptionType, Function, JsResult, Object, ProtoIntrinsics,
        builtins::{
            ordinary::ordinary_create_from_constructor, shared_array_buffer::SharedArrayBuffer,
        },
        create_shared_byte_data_block,
    },
    engine::context::{Bindable, GcScope},
};

mod shared_array_buffer_constructor;
mod shared_array_buffer_prototype;

pub(crate) use shared_array_buffer_constructor::*;
pub(crate) use shared_array_buffer_prototype::*;

/// ### [25.2.2.1 AllocateSharedArrayBuffer ( constructor, byteLength \[ , maxByteLength \] )](https://tc39.es/ecma262/#sec-allocatesharedarraybuffer)
///
/// The abstract operation AllocateSharedArrayBuffer takes arguments
/// constructor (a constructor) and byteLength (a non-negative integer) and
/// optional argument maxByteLength (a non-negative integer or empty) and
/// returns either a normal completion containing a SharedArrayBuffer or a
/// throw completion. It is used to create a SharedArrayBuffer.
fn allocate_shared_array_buffer<'a>(
    agent: &mut Agent,
    constructor: Object,
    byte_length: u64,
    max_byte_length: Option<u64>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, SharedArrayBuffer<'a>> {
    let constructor = constructor.bind(gc.nogc());
    // 1. Let slots be « [[ArrayBufferData]] ».
    // 2. If maxByteLength is present and maxByteLength is not empty, let
    //    allocatingGrowableBuffer be true; otherwise let allocatingGrowableBuffer
    //    be false.
    // 3. If allocatingGrowableBuffer is true, then
    // a. If byteLength > maxByteLength,
    if let Some(max_byte_length) = max_byte_length
        && byte_length > max_byte_length
    {
        // throw a RangeError exception.
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "byte length is larger than maximum byte length",
            gc.into_nogc(),
        ));
    }
    // b. Append [[ArrayBufferByteLengthData]] and
    //    [[ArrayBufferMaxByteLength]] to slots.
    // 4. Else,
    // a. Append [[ArrayBufferByteLength]] to slots.
    // 5. Let obj be ? OrdinaryCreateFromConstructor(constructor,
    //    "%SharedArrayBuffer.prototype%", slots).
    let Object::SharedArrayBuffer(obj) = ordinary_create_from_constructor(
        agent,
        Function::try_from(constructor).unwrap().unbind(),
        ProtoIntrinsics::SharedArrayBuffer,
        gc.reborrow(),
    )
    .unbind()?
    else {
        unreachable!()
    };
    let gc = gc.into_nogc();
    let obj = obj.bind(gc);
    // 6. If allocatingGrowableBuffer is true, let allocLength be
    //    maxByteLength; otherwise let allocLength be byteLength.
    // 7. Let block be ? CreateSharedByteDataBlock(allocLength).
    // 9. If allocatingGrowableBuffer is true, then
    // a. Assert: byteLength ≤ maxByteLength.
    // b. Let byteLengthBlock be ? CreateSharedByteDataBlock(8).
    // c. Perform SetValueInBuffer(byteLengthBlock, 0, biguint64,
    //    ℤ(byteLength), true, seq-cst).
    // d. Set obj.[[ArrayBufferByteLengthData]] to byteLengthBlock.
    // e. Set obj.[[ArrayBufferMaxByteLength]] to maxByteLength.
    // 10. Else,
    // a. Set obj.[[ArrayBufferByteLength]] to byteLength.
    // NOTE: create_shared_byte_data_block handles the maxByteLength logic.
    // SAFETY: 3.a. if byteLength > maxByteLength, throw a TypeError.
    let block = unsafe { create_shared_byte_data_block(agent, byte_length, max_byte_length, gc) }?;
    // 8. Set obj.[[ArrayBufferData]] to block.
    // SAFETY: we just created the obj; it cannot yet have any data block set.
    unsafe { obj.set_data_block(agent, block) };
    // 11. Return obj.
    Ok(obj)
}
