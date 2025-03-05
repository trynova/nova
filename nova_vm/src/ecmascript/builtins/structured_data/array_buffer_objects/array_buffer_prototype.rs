// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::type_conversion::try_to_index;
use crate::engine::TryResult;
use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::construct,
            type_conversion::{to_index, to_integer_or_infinity},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, ArrayBuffer, Behaviour, Builtin, BuiltinGetter,
            array_buffer::{is_detached_buffer, is_fixed_length_array_buffer},
        },
        execution::{Agent, JsResult, RealmIdentifier, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, IntoFunction, IntoValue, Object, PropertyKey, String, Value,
        },
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct ArrayBufferPrototype;

struct ArrayBufferPrototypeGetByteLength;
impl Builtin for ArrayBufferPrototypeGetByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteLength;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.byteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_byte_length);
}
impl BuiltinGetter for ArrayBufferPrototypeGetByteLength {}
struct ArrayBufferPrototypeGetDetached;
impl Builtin for ArrayBufferPrototypeGetDetached {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_detached;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.detached.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_detached);
}
impl BuiltinGetter for ArrayBufferPrototypeGetDetached {}
struct ArrayBufferPrototypeGetMaxByteLength;
impl Builtin for ArrayBufferPrototypeGetMaxByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_maxByteLength;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.maxByteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_max_byte_length);
}
impl BuiltinGetter for ArrayBufferPrototypeGetMaxByteLength {}
struct ArrayBufferPrototypeGetResizable;
impl Builtin for ArrayBufferPrototypeGetResizable {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_resizable;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.resizable.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_resizable);
}
impl BuiltinGetter for ArrayBufferPrototypeGetResizable {}
struct ArrayBufferPrototypeResize;
impl Builtin for ArrayBufferPrototypeResize {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.resize;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::resize);
}
struct ArrayBufferPrototypeSlice;
impl Builtin for ArrayBufferPrototypeSlice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::slice);
}
struct ArrayBufferPrototypeTransfer;
impl Builtin for ArrayBufferPrototypeTransfer {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.transfer;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::transfer);
}
struct ArrayBufferPrototypeTransferToFixedLength;
impl Builtin for ArrayBufferPrototypeTransferToFixedLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.transferToFixedLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::transfer_to_fixed_length);
}

impl ArrayBufferPrototype {
    /// ### [25.1.6.1 get ArrayBuffer.prototype.byteLength](https://tc39.es/ecma262/#sec-get-arraybuffer.prototype.bytelength)
    ///
    /// ArrayBuffer.prototype.byteLength is an accessor property whose set
    /// accessor function is undefined.
    fn get_byte_length<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is true, throw a TypeError exception.
        let o = require_internal_slot_array_buffer(agent, this_value, gc.nogc())?;
        // 4. If IsDetachedBuffer(O) is true, return +0𝔽.
        // 5. Let length be O.[[ArrayBufferByteLength]].
        // 6. Return 𝔽(length).

        // Note: byte_length takes detached status into account. The maximum
        // byte length of an ArrayBuffer is always within 2^53 - 1.
        Ok((o.byte_length(agent) as i64).try_into().unwrap())
    }

    /// ### [25.1.6.3 get ArrayBuffer.prototype.detached](https://tc39.es/ecma262/#sec-get-arraybuffer.prototype.detached)
    ///
    /// ArrayBuffer.prototype.detached is an accessor property whose set accessor function is undefined.
    fn get_detached<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is true, throw a TypeError exception.
        let o = require_internal_slot_array_buffer(agent, this_value, gc.nogc())?;
        // 4. Return IsDetachedBuffer(O).
        Ok(is_detached_buffer(agent, o).into())
    }

    /// ### [25.1.6.4 get ArrayBuffer.prototype.maxByteLength](https://tc39.es/ecma262/#sec-get-arraybuffer.prototype.maxbytelength)
    ///
    /// ArrayBuffer.prototype.maxByteLength is an accessor property whose set accessor function is undefined.
    fn get_max_byte_length<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is true, throw a TypeError exception.
        let o = require_internal_slot_array_buffer(agent, this_value, gc.nogc())?;
        // 4. If IsDetachedBuffer(O) is true, return +0𝔽.
        // 5. If IsFixedLengthArrayBuffer(O) is true, then
        // a. Let length be O.[[ArrayBufferByteLength]].
        // 6. Else,
        // a. Let length be O.[[ArrayBufferMaxByteLength]].
        // 7. Return 𝔽(length).
        Ok((o.max_byte_length(agent) as i64).try_into().unwrap())
    }

    /// ### [25.1.6.5 get ArrayBuffer.prototype.resizable](https://tc39.es/ecma262/#sec-get-arraybuffer.prototype.resizable)
    ///
    /// ArrayBuffer.prototype.resizable is an accessor property whose set accessor function is undefined.
    fn get_resizable<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is true, throw a TypeError exception.´
        let o = require_internal_slot_array_buffer(agent, this_value, gc.nogc())?;
        // 4. If IsFixedLengthArrayBuffer(O) is false, return true; otherwise return false.
        Ok((!is_fixed_length_array_buffer(agent, o)).into())
    }

    /// ### [25.1.6.6 ArrayBuffer.prototype.resize ( newLength )](https://tc39.es/ecma262/#sec-arraybuffer.prototype.resize)
    ///
    /// This method performs the following steps when called:
    fn resize<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let new_length = arguments.get(0);
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferMaxByteLength]]).
        // 3. If IsSharedArrayBuffer(O) is true, throw a TypeError exception.´
        let mut o = require_internal_slot_array_buffer(agent, this_value, gc.nogc())?;
        if !o.is_resizable(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Attempted to resize fixed length ArrayBuffer",
                gc.nogc(),
            ));
        }
        // 4. Let newByteLength be ? ToIndex(newLength).
        let new_byte_length =
            if let TryResult::Continue(res) = try_to_index(agent, new_length, gc.nogc()) {
                res? as usize
            } else {
                let scoped_o = o.scope(agent, gc.nogc());
                let res = to_index(agent, new_length, gc.reborrow())?;
                o = scoped_o.get(agent).bind(gc.nogc());
                res as usize
            };
        // 5. If IsDetachedBuffer(O) is true, throw a TypeError exception.
        if is_detached_buffer(agent, o) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot resize a detached ArrayBuffer",
                gc.nogc(),
            ));
        }
        // 6. If newByteLength > O.[[ArrayBufferMaxByteLength]], throw a RangeError exception.
        if new_byte_length > o.max_byte_length(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Attempted to resize beyond ArrayBuffer maxByteLength",
                gc.nogc(),
            ));
        }
        // 7. Let hostHandled be ? HostResizeArrayBuffer(O, newByteLength).
        // 8. If hostHandled is handled, return undefined.
        // TODO: HostResizeArrayBuffer

        // 9. Let oldBlock be O.[[ArrayBufferData]].
        // 10. Let newBlock be ? CreateByteDataBlock(newByteLength).
        // 11. Let copyLength be min(newByteLength, O.[[ArrayBufferByteLength]]).
        // 12. Perform CopyDataBlockBytes(newBlock, 0, oldBlock, 0, copyLength).
        // 13. NOTE: Neither creation of the new Data Block nor copying from
        // the old Data Block are observable. Implementations may implement
        // this method as in-place growth or shrinkage.
        // 14. Set O.[[ArrayBufferData]] to newBlock.
        // 15. Set O.[[ArrayBufferByteLength]] to newByteLength.
        o.resize(agent, new_byte_length);

        // 16. Return undefined.
        Ok(Value::Undefined)
    }

    /// ### [25.1.6.7 ArrayBuffer.prototype.slice ( start, end )](https://tc39.es/ecma262/#sec-arraybuffer.prototype.slice)
    ///
    /// This method performs the following steps when called:
    fn slice<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let start = arguments.get(0);
        let end = arguments.get(1);
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is true, throw a TypeError exception.´
        let o = require_internal_slot_array_buffer(agent, this_value, gc.nogc())?;
        // 4. If IsDetachedBuffer(O) is true, throw a TypeError exception.
        if is_detached_buffer(agent, o) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot slice a detached ArrayBuffer",
                gc.nogc(),
            ));
        }
        // 5. Let len be O.[[ArrayBufferByteLength]].
        let len = o.byte_length(agent);

        let scoped_o = o.scope(agent, gc.nogc());
        // 6. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start = to_integer_or_infinity(agent, start, gc.reborrow())?;
        // 7. If relativeStart = -∞, let first be 0.
        let first = if relative_start.is_neg_infinity() {
            0
        } else if relative_start.is_negative() {
            // 8. Else if relativeStart < 0, let first be max(len + relativeStart, 0).
            (len as i64 + relative_start.into_i64()).max(0) as usize
        } else {
            // 9. Else, let first be min(relativeStart, len).
            (relative_start.into_i64() as usize).min(len)
        };

        // 10. If end is undefined, let relativeEnd be len;
        let final_end = if end.is_undefined() {
            len
        } else {
            // else let relativeEnd be ? ToIntegerOrInfinity(end).
            let relative_end = to_integer_or_infinity(agent, end, gc.reborrow())?;
            // 11. If relativeEnd = -∞, let final be 0.
            if relative_end.is_neg_infinity() {
                0
            } else if relative_end.is_negative() {
                // 12. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
                (len as i64 + relative_end.into_i64()).max(0) as usize
            } else {
                // 13. Else, let final be min(relativeEnd, len).
                (relative_end.into_i64() as usize).min(len)
            }
        };

        // 14. Let newLen be max(final - first, 0).
        let new_len = (final_end as isize - first as isize).max(0) as usize;
        // 15. Let ctor be ? SpeciesConstructor(O, %ArrayBuffer%).
        let ctor = agent.current_realm().intrinsics().array_buffer();
        // 16. Let new be ? Construct(ctor, « 𝔽(newLen) »).
        let Object::ArrayBuffer(new) = construct(
            agent,
            ctor.into_function(),
            Some(ArgumentsList(&[(new_len as i64).try_into().unwrap()])),
            None,
            gc.reborrow(),
        )?
        .unbind() else {
            unreachable!();
        };
        let gc = gc.into_nogc();
        let new = new.bind(gc);
        // 17. Perform ? RequireInternalSlot(new, [[ArrayBufferData]]).
        // 18. If IsSharedArrayBuffer(new) is true, throw a TypeError exception.
        // 19. If IsDetachedBuffer(new) is true, throw a TypeError exception.
        if is_detached_buffer(agent, new) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Construction produced a detached ArrayBuffer",
                gc,
            ));
        }
        // 20. If SameValue(new, O) is true, throw a TypeError exception.
        let o = scoped_o.get(agent).bind(gc);
        if new == o {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Construction returned the original ArrayBuffer",
                gc,
            ));
        }
        // 21. If new.[[ArrayBufferByteLength]] < newLen, throw a TypeError exception.
        if new.byte_length(agent) < new_len {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Construction returned a smaller ArrayBuffer than requested",
                gc,
            ));
        }
        // 22. NOTE: Side-effects of the above steps may have detached or resized O.
        // 23. If IsDetachedBuffer(O) is true, throw a TypeError exception.
        if is_detached_buffer(agent, o) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Construction detached ArrayBuffer being sliced",
                gc,
            ));
        }
        // 24. Let fromBuf be O.[[ArrayBufferData]].
        // 25. Let toBuf be new.[[ArrayBufferData]].
        // 26. Let currentLen be O.[[ArrayBufferByteLength]].
        let current_len = o.byte_length(agent);
        // 27. If first < currentLen, then
        if first < current_len {
            // a. Let count be min(newLen, currentLen - first).
            let count = new_len.min(current_len - first);
            // b. Perform CopyDataBlockBytes(toBuf, 0, fromBuf, first, count).
            new.copy_array_buffer_data(agent, o, first, count);
        }
        // 28. Return new.
        Ok(new.into_value())
    }

    /// ### [25.1.6.8 ArrayBuffer.prototype.transfer ( [ newLength ] )](https://tc39.es/ecma262/#sec-arraybuffer.prototype.transfer)
    ///
    /// This method performs the following steps when called:
    fn transfer<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Return ? ArrayBufferCopyAndDetach(O, newLength, preserve-resizability).
        todo!()
    }

    /// ### [25.1.6.9 ArrayBuffer.prototype.transferToFixedLength ( [ newLength ] )](https://tc39.es/ecma262/#sec-arraybuffer.prototype.transfertofixedlength)
    ///
    /// This method performs the following steps when called:
    fn transfer_to_fixed_length<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Return ? ArrayBufferCopyAndDetach(O, newLength, fixed-length).
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.array_buffer_prototype();
        let array_buffer_constructor = intrinsics.array_buffer();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(10)
            .with_prototype(object_prototype)
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetByteLength>()
            .with_constructor_property(array_buffer_constructor)
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetDetached>()
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetMaxByteLength>()
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetResizable>()
            .with_builtin_function_property::<ArrayBufferPrototypeResize>()
            .with_builtin_function_property::<ArrayBufferPrototypeSlice>()
            .with_builtin_function_property::<ArrayBufferPrototypeTransfer>()
            .with_builtin_function_property::<ArrayBufferPrototypeTransferToFixedLength>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.ArrayBuffer.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[inline]
pub(crate) fn require_internal_slot_array_buffer<'a>(
    agent: &mut Agent,
    o: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<ArrayBuffer<'a>> {
    match o {
        // 1. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 2. If IsSharedArrayBuffer(O) is true, throw a TypeError exception.
        Value::ArrayBuffer(array_buffer) => Ok(array_buffer.unbind()),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected this to be ArrayBuffer",
            gc,
        )),
    }
}
