// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, ExceptionType, Function, JsResult, Object, Realm, String,
        Value, builders::BuiltinFunctionBuilder, temporal_err_to_js_err,
        to_integer_with_truncation,
    },
    engine::{Bindable as _, GcScope, NoGcScope, Scopable},
    heap::IntrinsicConstructorIndexes,
};

/// Constructor function object for %Temporal.PlainTime%.
pub(crate) struct TemporalPlainTimeConstructor;

impl Builtin for TemporalPlainTimeConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.PlainTime;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(TemporalPlainTimeConstructor::constructor);
}
impl BuiltinIntrinsicConstructor for TemporalPlainTimeConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalPlainTime;
}

impl TemporalPlainTimeConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let years = args.get(1).scope(agent, gc.nogc());
        let months = args.get(2).scope(agent, gc.nogc());
        let weeks = args.get(3).scope(agent, gc.nogc());
        let days = args.get(4).scope(agent, gc.nogc());
        let hours = args.get(5).scope(agent, gc.nogc());
        let minutes = args.get(6).scope(agent, gc.nogc());
        let seconds = args.get(7).scope(agent, gc.nogc());
        let milliseconds = args.get(8).scope(agent, gc.nogc());
        let microseconds = args.get(9).scope(agent, gc.nogc());
        let nanoseconds = args.get(10).scope(agent, gc.nogc());
        let new_target = new_target.bind(gc.nogc());

        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin Temporal.PlainTime constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };

        let Ok(new_target) = Function::try_from(new_target) else {
            unreachable!()
        };
        let new_target = new_target.scope(agent, gc.nogc());

        // 2. If hour is undefined, set hour to 0; else set hour to ? ToIntegerWithTruncation(hour).
        let h = if hours.get(agent).is_undefined() {
            Ok(0)
        } else {
            u8::try_from(
                to_integer_with_truncation(agent, hours.get(agent), gc.reborrow()).unbind()?,
            )
        };

        // 3. If minute is undefined, set minute to 0; else set minute to ? ToIntegerWithTruncation(minute).
        let m = if minutes.get(agent).is_undefined() {
            Ok(0)
        } else {
            u8::try_from(
                to_integer_with_truncation(agent, minutes.get(agent), gc.reborrow()).unbind()?,
            )
        };

        // 4. If second is undefined, set second to 0; else set second to ? ToIntegerWithTruncation(second).
        let s = if seconds.get(agent).is_undefined() {
            Ok(0)
        } else {
            u8::try_from(
                to_integer_with_truncation(agent, seconds.get(agent), gc.reborrow()).unbind()?,
            )
        };

        // 5. If millisecond is undefined, set millisecond to 0; else set millisecond to ? ToIntegerWithTruncation(millisecond).
        let ms = if milliseconds.get(agent).is_undefined() {
            Ok(0)
        } else {
            u16::try_from(
                to_integer_with_truncation(agent, milliseconds.get(agent), gc.reborrow())
                    .unbind()?,
            )
        };

        // 6. If microsecond is undefined, set microsecond to 0; else set microsecond to ? ToIntegerWithTruncation(microsecond).
        let mis = if microseconds.get(agent).is_undefined() {
            Ok(0)
        } else {
            u16::try_from(
                to_integer_with_truncation(agent, microseconds.get(agent), gc.reborrow())
                    .unbind()?,
            )
        };

        // 7. If nanosecond is undefined, set nanosecond to 0; else set nanosecond to ? ToIntegerWithTruncation(nanosecond).
        let ns = if nanoseconds.get(agent).is_undefined() {
            Ok(0)
        } else {
            u16::try_from(
                to_integer_with_truncation(agent, nanoseconds.get(agent), gc.reborrow())
                    .unbind()?,
            )
        };

        // 8. If IsValidTime(hour, minute, second, millisecond, microsecond, nanosecond) is false, throw a RangeError exception.

        // 9. Let time be CreateTimeRecord(hour, minute, second, millisecond, microsecond, nanosecond).
        let time = if let (
            Ok(hour),
            Ok(minute),
            Ok(second),
            Ok(millisecond),
            Ok(microsecond),
            Ok(nanosecond),
        ) = (h, m, s, ms, mis, ns)
        {
            temporal_rs::PlainTime::try_new(
                hour,
                minute,
                second,
                millisecond,
                microsecond,
                nanosecond,
            )
            .map_err(|err| temporal_err_to_js_err(agent, err, gc))
            .unbind()?
        } else {
            todo!() // TODO: create range error
        };

        // 10. Return ? CreateTemporalTime(time, NewTarget).
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _gc: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let plain_time_prototype = intrinsics.temporal_plain_time_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TemporalPlainTimeConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(plain_time_prototype.into())
        .build();
    }
}
