// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use temporal_rs::options::{RoundingMode, RoundingOptions, Unit};

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter,
        ExceptionType, JsResult, PropertyKey, Realm, String, StringOptionType, Value,
        builders::OrdinaryObjectBuilder,
        builtins::temporal::plain_time::{
            add_duration_to_time, require_internal_slot_temporal_plain_time,
        },
        create_temporal_plain_time, get_options_object, get_rounding_increment_option,
        get_rounding_mode_option, get_temporal_unit_valued_option, temporal_err_to_js_err,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable},
    heap::WellKnownSymbols,
};

pub(crate) struct TemporalPlainTimePrototype;

struct TemporalPlainTimePrototypeGetHour;
impl Builtin for TemporalPlainTimePrototypeGetHour {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_hour;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.hour.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::get_hour);
}
impl BuiltinGetter for TemporalPlainTimePrototypeGetHour {}

struct TemporalPlainTimePrototypeGetMinute;
impl Builtin for TemporalPlainTimePrototypeGetMinute {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_minute;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.minute.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::get_minute);
}
impl BuiltinGetter for TemporalPlainTimePrototypeGetMinute {}

struct TemporalPlainTimePrototypeGetSecond;
impl Builtin for TemporalPlainTimePrototypeGetSecond {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_second;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.second.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::get_second);
}
impl BuiltinGetter for TemporalPlainTimePrototypeGetSecond {}

struct TemporalPlainTimePrototypeGetMillisecond;
impl Builtin for TemporalPlainTimePrototypeGetMillisecond {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_millisecond;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.millisecond.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::get_millisecond);
}
impl BuiltinGetter for TemporalPlainTimePrototypeGetMillisecond {}
struct TemporalPlainTimePrototypeGetMicrosecond;
impl Builtin for TemporalPlainTimePrototypeGetMicrosecond {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_microsecond;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.microsecond.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::get_microsecond);
}
impl BuiltinGetter for TemporalPlainTimePrototypeGetMicrosecond {}

struct TemporalPlainTimePrototypeGetNanosecond;
impl Builtin for TemporalPlainTimePrototypeGetNanosecond {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_nanosecond;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.nanosecond.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::get_nanosecond);
    const WRITABLE: bool = true;
    const ENUMERABLE: bool = false;
    const CONFIGURABLE: bool = true;
}
impl BuiltinGetter for TemporalPlainTimePrototypeGetNanosecond {}

struct TemporalPlainTimePrototypeAdd;
impl Builtin for TemporalPlainTimePrototypeAdd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.add;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::add);
}

struct TemporalPlainTimePrototypeSubtract;
impl Builtin for TemporalPlainTimePrototypeSubtract {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.subtract;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::subtract);
}

struct TemporalPlainTimePrototypeRound;
impl Builtin for TemporalPlainTimePrototypeRound {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.round;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::round);
}
impl TemporalPlainTimePrototype {
    /// ### [4.3.4 get Temporal.PlainTime.prototype.minute](https://tc39.es/proposal-temporal/#sec-get-temporal.plaintime.prototype.minute)
    pub(crate) fn get_minute<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let plainTime be the this value.
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, this_value, gc)?;
        // 3. Return 𝔽(plainTime.[[Time]].[[Minute]]).
        let value = plain_time.inner_plain_time(agent).minute();
        Ok(value.into())
    }
    /// ### [4.3.5 get Temporal.PlainTime.prototype.second](https://tc39.es/proposal-temporal/#sec-get-temporal.plaintime.prototype.second)
    pub(crate) fn get_second<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let plainTime be the this value.
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, this_value, gc)?;
        // 3. Return 𝔽(plainTime.[[Time]].[[Second]]).
        let value = plain_time.inner_plain_time(agent).second();
        Ok(value.into())
    }

    /// ### [4.3.6 get Temporal.PlainTime.prototype.millisecond](https://tc39.es/proposal-temporal/#sec-get-temporal.plaintime.prototype.millisecond)
    pub(crate) fn get_millisecond<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let plainTime be the this value.
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, this_value, gc)?;
        // 3. Return 𝔽(plainTime.[[Time]].[[Millisecond]]).
        let value = plain_time.inner_plain_time(agent).millisecond();
        Ok(value.into())
    }

    /// ### [4.3.8 get Temporal.PlainTime.prototype.nanosecond](https://tc39.es/proposal-temporal/#sec-get-temporal.plaintime.prototype.nanosecond)
    pub(crate) fn get_nanosecond<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let plainTime be the this value.
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, this_value, gc)?;
        // 3. Return 𝔽(plainTime.[[Time]].[[Nanosecond]]).
        let value = plain_time.inner_plain_time(agent).nanosecond();
        Ok(value.into())
    }

    // ### [4.3.3 get Temporal.PlainTime.prototype.hour](https://tc39.es/proposal-temporal/#sec-get-temporal.plaintime.prototype.hour)
    pub(crate) fn get_hour<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let plainTime be the this value.
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, this_value, gc)?;
        // 3. Return 𝔽(plainTime.[[Time]].[[Hour]]).
        let value = plain_time.inner_plain_time(agent).hour();
        Ok(value.into())
    }

    /// ### [4.3.4 get Temporal.PlainTime.prototype.microsecond](https://tc39.es/proposal-temporal/#sec-get-temporal.plaintime.prototype.microsecond)
    pub(crate) fn get_microsecond<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let plainTime be the this value.
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, this_value, gc)?;
        // 3. Return 𝔽(plainTime.[[Time]].[[Microsecond]]).
        let value = plain_time.inner_plain_time(agent).microsecond();
        Ok(value.into())
    }

    /// ### [4.3.9 Temporal.PlainTime.prototype.add ( temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal.plaintime.prototype.add)
    fn add<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let duration = args.get(0).bind(gc.nogc());
        // 1. Let plainTime be the this value.
        let plain_time = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time =
            require_internal_slot_temporal_plain_time(agent, plain_time.unbind(), gc.nogc())
                .unbind()?
                .bind(gc.nogc());
        // 3. Return ? AddDurationToTime(add, plainTime, temporalDurationLike).
        const SUBTRACT: bool = true;
        add_duration_to_time::<SUBTRACT>(agent, plain_time.unbind(), duration.unbind(), gc)
            .map(Value::from)
    }

    /// ### [4.3.10 Temporal.PlainTime.prototype.subtract ( temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal.plaintime.prototype.subtract)
    fn subtract<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let duration = args.get(0).bind(gc.nogc());
        // 1. Let plainTime be the this value.
        let plain_time = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time =
            require_internal_slot_temporal_plain_time(agent, plain_time.unbind(), gc.nogc())
                .unbind()?
                .bind(gc.nogc());
        // 3. Return ? AddDurationToTime(subtract, plainTime, temporalDurationLike).
        const ADD: bool = false;
        add_duration_to_time::<ADD>(agent, plain_time.unbind(), duration.unbind(), gc)
            .map(Value::from)
    }

    /// ### [5.3.14 Temporal.PlainTime.prototype.round ( roundTo )](https://tc39.es/proposal-temporal/#sec-temporal.plaintime.prototype.round)
    fn round<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let round_to = args.get(0).bind(gc.nogc());
        // 1. Let plainTime be the this value.
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 3. If roundTo is undefined, throw a TypeError exception.
        if round_to.is_undefined() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "rountTo cannot be undefined",
                gc.into_nogc(),
            ));
        }

        // 4. If roundTo is a String, then
        let options = if let Ok(round_to) = String::try_from(round_to) {
            //        a. Let paramString be roundTo.
            //        b. Set roundTo to OrdinaryObjectCreate(null).
            //        c. Perform ! CreateDataPropertyOrThrow(roundTo, "smallestUnit", paramString).
            let mut options = RoundingOptions::default();
            options.smallest_unit =
                Some(Unit::from_string(agent, round_to.unbind(), gc.nogc()).unbind()?);
            options
        } else {
            // 5. Else,
            // a. Set roundTo to ? GetOptionsObject(roundTo).
            let round_to = get_options_object(agent, round_to, gc.nogc())
                .unbind()?
                .map(|r| r.scope(agent, gc.nogc()));
            // 6. NOTE: The following steps read options and perform independent validation in alphabetical
            // order (GetRoundingIncrementOption reads "roundingIncrement" and GetRoundingModeOption reads "roundingMode").
            let mut options = RoundingOptions::default();

            let (rounding_increment, rounding_mode, smallest_unit) =
                if let Some(round_to) = round_to {
                    // 7. Let roundingIncrement be ? GetRoundingIncrementOption(roundTo).
                    let rounding_increment =
                        get_rounding_increment_option(agent, round_to.get(agent), gc.reborrow())
                            .unbind()?;
                    // 8. Let roundingMode be ? GetRoundingModeOption(roundTo, half-expand).
                    let rounding_mode = get_rounding_mode_option(
                        agent,
                        round_to.get(agent),
                        RoundingMode::default(),
                        gc.reborrow(),
                    )
                    .unbind()?;
                    // 9. Let smallestUnit be ? GetTemporalUnitValuedOption(roundTo, "smallestUnit", required).
                    let smallest_unit = get_temporal_unit_valued_option(
                        agent,
                        round_to.get(agent),
                        BUILTIN_STRING_MEMORY.smallestUnit.into(),
                        gc.reborrow(),
                    )
                    .unbind()?;
                    (rounding_increment, rounding_mode, smallest_unit)
                } else {
                    Default::default()
                };

            options.increment = Some(rounding_increment);
            options.rounding_mode = Some(rounding_mode);
            options.smallest_unit = smallest_unit;
            options
        };

        // 10. Perform ? ValidateTemporalUnitValue(smallestUnit, time).
        // 11. Let maximum be MaximumTemporalDurationRoundingIncrement(smallestUnit).
        // 12. Assert: maximum is not unset.
        // 13. Perform ? ValidateTemporalRoundingIncrement(roundingIncrement, maximum, false).
        // 14. Let result be RoundTime(plainTime.[[Time]], roundingIncrement, smallestUnit, roundingMode).
        let result = plain_time
            .get(agent)
            .inner_plain_time(agent)
            .round(options)
            .map_err(|e| temporal_err_to_js_err(agent, e, gc.nogc()))
            .unbind()?;
        // 15. Return ! CreateTemporalTime(result).
        Ok(create_temporal_plain_time(agent, result, None, gc)
            .unwrap()
            .into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_plain_time_prototype();
        let object_prototype = intrinsics.object_prototype();
        let plain_time_constructor = intrinsics.temporal_plain_time();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(10)
            .with_prototype(object_prototype)
            .with_constructor_property(plain_time_constructor)
            .with_builtin_function_getter_property::<TemporalPlainTimePrototypeGetHour>()
            .with_builtin_function_getter_property::<TemporalPlainTimePrototypeGetMinute>()
            .with_builtin_function_getter_property::<TemporalPlainTimePrototypeGetSecond>()
            .with_builtin_function_getter_property::<TemporalPlainTimePrototypeGetMicrosecond>()
            .with_builtin_function_getter_property::<TemporalPlainTimePrototypeGetNanosecond>()
            .with_builtin_function_getter_property::<TemporalPlainTimePrototypeGetMillisecond>()
            .with_builtin_function_property::<TemporalPlainTimePrototypeAdd>()
            .with_builtin_function_property::<TemporalPlainTimePrototypeSubtract>()
            .with_builtin_function_property::<TemporalPlainTimePrototypeRound>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbols::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal_PlainTime.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
