// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ecmascript_atomics::Ordering;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{construct, species_constructor},
            type_conversion::{
                to_index, to_integer_or_infinity, try_to_index, try_to_integer_or_infinity,
            },
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter,
            shared_array_buffer::SharedArrayBuffer,
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics, Realm,
            agent::{ExceptionType, GrowSharedArrayBufferResult, try_result_into_js},
        },
        types::{
            BUILTIN_STRING_MEMORY, Number, PropertyKey, String, Value, copy_shared_data_block_bytes,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SharedArrayBufferPrototype;

struct SharedArrayBufferPrototypeGetByteLength;
impl Builtin for SharedArrayBufferPrototypeGetByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteLength;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.byteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::get_byte_length);
}
impl BuiltinGetter for SharedArrayBufferPrototypeGetByteLength {}
struct SharedArrayBufferPrototypeGrow;
impl Builtin for SharedArrayBufferPrototypeGrow {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.grow;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::grow);
}
struct SharedArrayBufferPrototypeGetGrowable;
impl Builtin for SharedArrayBufferPrototypeGetGrowable {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_growable;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.growable.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::get_growable);
}
impl BuiltinGetter for SharedArrayBufferPrototypeGetGrowable {}
struct SharedArrayBufferPrototypeGetMaxByteLength;
impl Builtin for SharedArrayBufferPrototypeGetMaxByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_maxByteLength;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.maxByteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(SharedArrayBufferPrototype::get_max_byte_length);
}
impl BuiltinGetter for SharedArrayBufferPrototypeGetMaxByteLength {}
struct SharedArrayBufferPrototypeSlice;
impl Builtin for SharedArrayBufferPrototypeSlice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::slice);
}

impl SharedArrayBufferPrototype {
    /// ### [25.2.5.1 get SharedArrayBuffer.prototype.byteLength](https://tc39.es/ecma262/#sec-get-sharedarraybuffer.prototype.bytelength)
    ///
    /// SharedArrayBuffer.prototype.byteLength is an accessor property whose
    /// set accessor function is undefined.
    fn get_byte_length<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        let o = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is false, throw a TypeError exception.
        let o = require_internal_slot_shared_array_buffer(agent, o, gc)?;
        // 4. Let length be ArrayBufferByteLength(O, seq-cst).
        let length = o.byte_length(agent, Ordering::SeqCst);
        // 5. Return 𝔽(length).
        Ok(Number::from_i64(agent, length as i64, gc).into())
    }

    /// ### [25.2.5.3 SharedArrayBuffer.prototype.grow ( newLength )](https://tc39.es/ecma262/#sec-sharedarraybuffer.prototype.grow)
    fn grow<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let args = args.bind(gc.nogc());
        let new_length = args.get(0);
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferMaxByteLength]]).
        // 3. If IsSharedArrayBuffer(O) is false, throw a TypeError exception.
        let mut o = require_internal_slot_shared_array_buffer(agent, o, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        if !o.is_growable(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected this to be growable SharedArrayBuffer",
                gc.into_nogc(),
            ));
        }
        // 4. Let newByteLength be ? ToIndex(newLength).
        let new_byte_length = if let Some(n) =
            try_result_into_js(try_to_index(agent, new_length, gc.nogc()))
                .unbind()?
                .bind(gc.nogc())
        {
            n
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let n = to_index(agent, new_length.unbind(), gc.reborrow()).unbind()?;
            o = unsafe { scoped_o.take(agent) }.bind(gc.nogc());
            n
        };
        let o = o.unbind();
        let gc = gc.into_nogc();
        let o = o.bind(gc);
        // 5. Let hostHandled be ? HostGrowSharedArrayBuffer(O, newByteLength).
        let host_handled =
            agent
                .host_hooks
                .grow_shared_array_buffer(agent, o, new_byte_length, gc)?;
        // 6. If hostHandled is handled, return undefined.
        if host_handled == GrowSharedArrayBufferResult::Handled {
            return Ok(Value::Undefined);
        }
        // 11. Repeat,
        // a. NOTE: This is a compare-and-exchange loop to ensure that
        //    parallel, racing grows of the same buffer are totally ordered,
        //    are not lost, and do not silently do nothing. The loop exits if
        //    it was able to attempt to grow uncontended.
        // c. If newByteLength = currentByteLength, return undefined.
        // d. If newByteLength < currentByteLength or
        //    newByteLength > O.[[ArrayBufferMaxByteLength]], throw a
        //    RangeError exception.
        o.grow(agent, new_byte_length, gc).map(|_| Value::Undefined)
    }

    /// ### [25.2.5.4 get SharedArrayBuffer.prototype.growable](https://tc39.es/ecma262/#sec-get-sharedarraybuffer.prototype.growable)
    ///
    /// SharedArrayBuffer.prototype.growable is an accessor property whose set
    /// accessor function is undefined.
    fn get_growable<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        let o = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is false, throw a TypeError exception.
        let o = require_internal_slot_shared_array_buffer(agent, o, gc)?;
        // 4. If IsFixedLengthArrayBuffer(O) is false, return true; otherwise
        //    return false.
        Ok(o.is_growable(agent).into())
    }

    /// ### [25.2.5.5 get SharedArrayBuffer.prototype.maxByteLength](https://tc39.es/ecma262/#sec-get-sharedarraybuffer.prototype.maxbytelength)
    ///
    /// SharedArrayBuffer.prototype.maxByteLength is an accessor property whose
    /// set accessor function is undefined.
    fn get_max_byte_length<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let O be the this value.
        let o = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is false, throw a TypeError exception.
        let o = require_internal_slot_shared_array_buffer(agent, o, gc)?;
        // 4. If IsFixedLengthArrayBuffer(O) is true, then
        // a. Let length be O.[[ArrayBufferByteLength]].
        // 5. Else,
        // a. Let length be O.[[ArrayBufferMaxByteLength]].
        // 6. Return 𝔽(length).
        let length = o.max_byte_length(agent);
        // 5. Return 𝔽(length).
        Ok(Number::from_i64(agent, length as i64, gc).into())
    }

    /// ### [25.2.5.6 SharedArrayBuffer.prototype.slice ( start, end )](https://tc39.es/ecma262/#sec-hostgrowsharedarraybuffer)
    fn slice<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let args = args.bind(gc.nogc());
        let start = args.get(0);
        let mut end = args.get(1);
        // 1. Let O be the this value.
        let o = this_value;
        // 2. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 3. If IsSharedArrayBuffer(O) is false, throw a TypeError exception.
        let o = require_internal_slot_shared_array_buffer(agent, o, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 4. Let len be ArrayBufferByteLength(O, seq-cst).
        let len = o.byte_length(agent, Ordering::SeqCst);
        let o = o.scope(agent, gc.nogc());
        // 5. Let relativeStart be ? ToIntegerOrInfinity(start).
        let relative_start = if let Some(r) =
            try_result_into_js(try_to_integer_or_infinity(agent, start, gc.nogc())).unbind()?
        {
            r
        } else {
            let scoped_end = end.scope(agent, gc.nogc());
            let r = to_integer_or_infinity(agent, start.unbind(), gc.reborrow()).unbind()?;
            end = unsafe { scoped_end.take(agent) }.bind(gc.nogc());
            r
        };
        // 6. If relativeStart = -∞, let first be 0.
        let first = if relative_start.is_neg_infinity() {
            0
        } else if relative_start.is_negative() {
            // 7. Else if relativeStart < 0, let first be max(len + relativeStart, 0).
            (len as u64).saturating_sub(relative_start.into_i64().unsigned_abs()) as usize
        } else {
            // 8. Else, let first be min(relativeStart, len).
            relative_start.into_i64().min(len as i64) as usize
        };
        // 9. If end is undefined, let relativeEnd be len;
        let final_end = if end.is_undefined() {
            len
        } else {
            // else let relativeEnd be ? ToIntegerOrInfinity(end).
            let relative_end =
                to_integer_or_infinity(agent, end.unbind(), gc.reborrow()).unbind()?;
            // 10. If relativeEnd = -∞, let final be 0.
            if relative_end.is_neg_infinity() {
                0
            } else if relative_end.is_negative() {
                // 11. Else if relativeEnd < 0, let final be max(len + relativeEnd, 0).
                (len as u64).saturating_sub(relative_end.into_i64().unsigned_abs()) as usize
            } else {
                // 12. Else, let final be min(relativeEnd, len).
                relative_end.into_i64().min(len as i64) as usize
            }
        };
        // 13. Let newLen be max(final - first, 0).
        let new_len = final_end.saturating_sub(first);

        // 14. Let ctor be ? SpeciesConstructor(O, %SharedArrayBuffer%).
        let ctor = species_constructor(
            agent,
            o.get(agent).into(),
            ProtoIntrinsics::SharedArrayBuffer,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 15. Let new be ? Construct(ctor, « 𝔽(newLen) »).
        let new = {
            let mut new_len = Number::from_i64(agent, new_len as i64, gc.nogc())
                .into()
                .unbind();
            let args = ArgumentsList::from_mut_value(&mut new_len);
            construct(agent, ctor.unbind(), Some(args), None, gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };
        let new = new.unbind();
        let gc = gc.into_nogc();
        let new = new.bind(gc);
        // SAFETY: not shared.
        let o = unsafe { o.take(agent).bind(gc) };
        // 16. Perform ? RequireInternalSlot(new, [[ArrayBufferData]]).
        // 17. If IsSharedArrayBuffer(new) is false, throw a TypeError exception.
        let new = require_internal_slot_shared_array_buffer(agent, new.into(), gc)?;
        // 18. If new.[[ArrayBufferData]] is O.[[ArrayBufferData]],
        if new.get_data_block(agent) == o.get_data_block(agent) {
            // throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "SharedArrayBuffer subclass returned this from species constructor",
                gc,
            ));
        }
        // 19. If ArrayBufferByteLength(new, seq-cst) < newLen,
        if new.byte_length(agent, Ordering::SeqCst) < new_len {
            // throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "SharedArrayBuffer subclass returned smaller buffer from species constructor",
                gc,
            ));
        }
        // 20. Let fromBuf be O.[[ArrayBufferData]].
        let from_buf = o.get_data_block(agent);
        // 21. Let toBuf be new.[[ArrayBufferData]].
        let to_buf = new.get_data_block(agent);
        // 22. Perform CopyDataBlockBytes(toBuf, 0, fromBuf, first, newLen).
        copy_shared_data_block_bytes(to_buf, 0, from_buf, first, new_len);
        // 23. Return new.
        Ok(new.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.shared_array_buffer_prototype();
        let shared_array_buffer_constructor = intrinsics.shared_array_buffer();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(7)
            .with_prototype(object_prototype)
            .with_builtin_function_getter_property::<SharedArrayBufferPrototypeGetByteLength>()
            .with_constructor_property(shared_array_buffer_constructor)
            .with_builtin_function_property::<SharedArrayBufferPrototypeGrow>()
            .with_builtin_function_getter_property::<SharedArrayBufferPrototypeGetGrowable>()
            .with_builtin_function_getter_property::<SharedArrayBufferPrototypeGetMaxByteLength>()
            .with_builtin_function_property::<SharedArrayBufferPrototypeSlice>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.SharedArrayBuffer.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[inline]
pub(crate) fn require_internal_slot_shared_array_buffer<'a>(
    agent: &mut Agent,
    o: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, SharedArrayBuffer<'a>> {
    match o {
        // 1. Perform ? RequireInternalSlot(O, [[ArrayBufferData]]).
        // 2. If IsSharedArrayBuffer(O) is false, throw a TypeError exception.
        Value::SharedArrayBuffer(sab) => Ok(sab.unbind()),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected this to be SharedArrayBuffer",
            gc,
        )),
    }
}
