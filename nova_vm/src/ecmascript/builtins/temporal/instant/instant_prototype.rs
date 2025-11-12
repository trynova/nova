use std::num::NonZeroU32;

use temporal_rs::options::{RoundingIncrement, RoundingMode, RoundingOptions, Unit};

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{get, try_create_data_property_or_throw},
            type_conversion::to_number,
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin,
            ordinary::ordinary_object_create_with_intrinsics,
            temporal::{
                error::temporal_err_to_js_err,
                instant::{
                    add_duration_to_instant, create_temporal_instant, difference_temporal_instant,
                    require_internal_slot_temporal_instant, to_temporal_instant,
                },
            },
        },
        execution::{
            Agent, JsResult, Realm,
            agent::{ExceptionType, unwrap_try},
        },
        types::{BUILTIN_STRING_MEMORY, BigInt, IntoValue, Object, PropertyKey, String, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, trivially_bindable},
        rootable::Scopable,
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct TemporalInstantPrototype;

struct TemporalInstantPrototypeGetEpochMilliseconds;
impl Builtin for TemporalInstantPrototypeGetEpochMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getEpochMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(TemporalInstantPrototype::get_epoch_milliseconds);
}

struct TemporalInstantPrototypeGetEpochNanoSeconds;
impl Builtin for TemporalInstantPrototypeGetEpochNanoSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getEpochNanoSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(TemporalInstantPrototype::get_epoch_nanoseconds);
}

struct TemporalInstantPrototypeAdd;
impl Builtin for TemporalInstantPrototypeAdd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.add;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::add);
}

struct TemporalInstantPrototypeSubtract;
impl Builtin for TemporalInstantPrototypeSubtract {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.subtract;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::subtract);
}

struct TemporalInstantPrototypeUntil;
impl Builtin for TemporalInstantPrototypeUntil {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.until;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::until);
}

struct TemporalInstantPrototypeSince;
impl Builtin for TemporalInstantPrototypeSince {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.since;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::since);
}

struct TemporalInstantPrototypeRound;
impl Builtin for TemporalInstantPrototypeRound {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.round;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::round);
}

struct TemporalInstantPrototypeEquals;
impl Builtin for TemporalInstantPrototypeEquals {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.equals;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::equals);
}

struct TemporalInstantPrototypeToString;
impl Builtin for TemporalInstantPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::to_string);
}

struct TemporalInstantPrototypeToLocaleString;
impl Builtin for TemporalInstantPrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::to_locale_string);
}

struct TemporalInstantPrototypeToJSON;
impl Builtin for TemporalInstantPrototypeToJSON {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toJSON;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::to_json);
}

struct TemporalInstantPrototypeValueOf;
impl Builtin for TemporalInstantPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantPrototype::value_of);
}

struct TemporalInstantPrototypeToZonedDateTimeISO;
impl Builtin for TemporalInstantPrototypeToZonedDateTimeISO {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toZonedDateTimeISO;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(TemporalInstantPrototype::to_zoned_date_time_iso);
}

impl TemporalInstantPrototype {
    /// ### [8.3.3 get Temporal.Instant.prototype.epochMilliseconds](https://tc39.es/proposal-temporal/#sec-get-temporal.instant.prototype.epochmilliseconds)
    fn get_epoch_milliseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, this_value, gc)?;
        // 3. Let ns be instant.[[EpochNanoseconds]].
        // 4. Let ms be floor(ℝ(ns) / 10**6).
        // 5. Return 𝔽(ms).
        let value = instant.inner_instant(agent).epoch_milliseconds();
        Ok(Value::from_i64(agent, value, gc))
    }

    /// ### [8.3.4 get Temporal.Instant.prototype.epochNanoseconds](https://tc39.es/proposal-temporal/#sec-get-temporal.instant.prototype.epochnanoseconds)
    fn get_epoch_nanoseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, this_value, gc)?;
        // 3. Return instant.[[EpochNanoseconds]].
        let value = instant.inner_instant(agent).epoch_nanoseconds().as_i128();
        Ok(BigInt::from_i128(agent, value).into())
    }

    /// ### [8.3.5 Temporal.Instant.prototype.add ( temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.add)
    fn add<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let duration = args.get(0).bind(gc.nogc());
        // 1. Let instant be the this value.
        let instant = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, instant.unbind(), gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Return ? AddDurationToInstant(add, instant, temporalDurationLike).
        const ADD: bool = true;
        let result = add_duration_to_instant::<ADD>(agent, instant.unbind(), duration.unbind(), gc)
            .unbind()?;
        Ok(result.into_value())
    }

    /// ### [8.3.6 Temporal.Instant.prototype.subtract ( temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.subtract)
    fn subtract<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let duration = args.get(0).bind(gc.nogc());
        // 1. Let instant be the this value.
        let instant = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, instant.unbind(), gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Return ? AddDurationToInstant(subtract, instant, temporalDurationLike).
        const SUBTRACT: bool = false;
        let result =
            add_duration_to_instant::<SUBTRACT>(agent, instant.unbind(), duration.unbind(), gc)
                .unbind()?;
        Ok(result.into_value())
    }

    /// ### [8.3.7 Temporal.Instant.prototype.until ( other [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.until)
    fn until<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let other = args.get(0).bind(gc.nogc());
        let options = args.get(1).bind(gc.nogc());
        // 1. Let instant be the this value.
        let instant = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, instant.unbind(), gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Return ? DifferenceTemporalInstant(until, instant, other, options).
        const UNTIL: bool = true;
        let result = difference_temporal_instant::<UNTIL>(
            agent,
            instant.into_value().unbind(),
            other.unbind(),
            options.unbind(),
            gc,
        )
        .unbind()?;
        Ok(result.into_value())
    }

    /// ### [8.3.8 Temporal.Instant.prototype.since ( other [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.until)
    fn since<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let other = args.get(0).bind(gc.nogc());
        let options = args.get(1).bind(gc.nogc());
        // 1. Let instant be the this value.
        let instant = this_value;
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, instant.unbind(), gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Return ? DifferenceTemporalInstant(since, instant, other, options).
        const SINCE: bool = false;
        let result = difference_temporal_instant::<SINCE>(
            agent,
            instant.into_value().unbind(),
            other.unbind(),
            options.unbind(),
            gc,
        )
        .unbind()?;
        Ok(result.into_value())
    }

    /// ### [8.3.9 Temporal.Instant.prototype.round ( roundTo )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.round)
    fn round<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let this_value = this_value.bind(gc.nogc());
        let round_to = args.get(0).bind(gc.nogc());
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());

        // 3. If roundTo is undefined, then
        if round_to.unbind().is_undefined() {
            // a. Throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "roundTo cannot be undefined",
                gc.into_nogc(),
            ));
        }

        // 4. If roundTo is a String, then
        let round_to = if let Value::String(round_to) = round_to.unbind() {
            // a. Let paramString be roundTo.
            let param_string = round_to;
            // b. Set roundTo to OrdinaryObjectCreate(null).
            let round_to = ordinary_object_create_with_intrinsics(agent, None, None, gc.nogc());
            // c. Perform ! CreateDataPropertyOrThrow(roundTo, "smallestUnit", paramString).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                round_to,
                BUILTIN_STRING_MEMORY.smallestUnit.into(),
                param_string.into_value(),
                None,
                gc.nogc(),
            ));
            round_to
        } else {
            // 5. Else, set roundTo to ? GetOptionsObject(roundTo).
            get_options_object(agent, round_to.unbind(), gc.nogc())
                .unbind()?
                .bind(gc.nogc())
        };
        let round_to = round_to.scope(agent, gc.nogc());
        // 6. NOTE: The following steps read options and perform independent validation in
        //    alphabetical order (GetRoundingIncrementOption reads "roundingIncrement" and
        //    GetRoundingModeOption reads "roundingMode").
        let mut options = RoundingOptions::default();
        // 7. Let roundingIncrement be ? GetRoundingIncrementOption(roundTo).
        let rounding_increment =
            get_rounding_increment_option(agent, round_to.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
        options.increment = Some(rounding_increment);
        // 8. Let roundingMode be ? GetRoundingModeOption(roundTo, half-expand).
        let rounding_mode = get_rounding_mode_option(
            agent,
            round_to.get(agent),
            RoundingMode::default(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        options.rounding_mode = Some(rounding_mode);
        // 9. Let smallestUnit be ? GetTemporalUnitValuedOption(roundTo, "smallestUnit", required).
        let smallest_unit = get_temporal_unit_valued_option(
            agent,
            round_to.get(agent),
            BUILTIN_STRING_MEMORY.smallestUnit.into(),
            DefaultOption::Required,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        options.smallest_unit = Some(smallest_unit);
        // 10. Perform ? ValidateTemporalUnitValue(smallestUnit, time).
        // 11. If smallestUnit is hour, then
        //     a. Let maximum be HoursPerDay.
        // 12. Else if smallestUnit is minute, then
        //     a. Let maximum be MinutesPerHour × HoursPerDay.
        // 13. Else if smallestUnit is second, then
        //     a. Let maximum be SecondsPerMinute × MinutesPerHour × HoursPerDay.
        // 14. Else if smallestUnit is millisecond, then
        //     a. Let maximum be ℝ(msPerDay).
        // 15. Else if smallestUnit is microsecond, then
        //     a. Let maximum be 10**3 × ℝ(msPerDay).
        // 16. Else,
        //     a. Assert: smallestUnit is nanosecond.
        //     b. Let maximum be nsPerDay.
        // 17. Perform ? ValidateTemporalRoundingIncrement(roundingIncrement, maximum, true).
        // 18. Let roundedNs be RoundTemporalInstant(instant.[[EpochNanoseconds]], roundingIncrement, smallestUnit, roundingMode).
        let rounded_ns = instant
            .get(agent)
            .inner_instant(agent)
            .round(options)
            .map_err(|e| temporal_err_to_js_err(agent, e, gc.nogc()))
            .unbind()?
            .bind(gc.nogc());
        // 19. Return ! CreateTemporalInstant(roundedNs).
        Ok(create_temporal_instant(agent, rounded_ns, None, gc)?.into_value())
    }

    /// ### [8.3.10 Temporal.Instant.prototype.equals ( other )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.equals)
    fn equals<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let other = args.get(0).bind(gc.nogc());
        let this_value = this_value.bind(gc.nogc());
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, this_value.unbind(), gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 3. Set other to ? ToTemporalInstant(other).
        let other_instant = to_temporal_instant(agent, other.unbind(), gc.reborrow()).unbind()?;
        // 4. If instant.[[EpochNanoseconds]] ≠ other.[[EpochNanoseconds]], return false.
        let instant_val = instant.get(agent).bind(gc.nogc());
        if *instant_val.inner_instant(agent) != other_instant {
            return Ok(Value::from(false));
        }
        // 5. Return true.
        Ok(Value::from(true))
    }

    /// ### [8.3.11 Temporal.Instant.prototype.toString ( [ options ] )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.tostring)
    fn to_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    /// ### [8.3.12 Temporal.Instant.prototype.toLocaleString ( [ locales [ , options ] ] )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.tolocalestring)
    fn to_locale_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    /// ###[8.3.13 Temporal.Instant.prototype.toJSON ( )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.tojson)
    fn to_json<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    /// ###[8.3.14 Temporal.Instant.prototype.valueOf ( )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.valueof)
    /// Note:
    ///     This method always throws, because in the absence of valueOf(), expressions with
    ///     arithmetic operators such as instant1 > instant2 would fall back to being equivalent
    ///     to instant1.toString() > instant2.toString(). Lexicographical comparison of
    ///     serialized strings might not seem obviously wrong, because the result would
    ///     sometimes be correct. Implementations are encouraged to phrase the error message to
    ///     point users to Temporal.Instant.compare(), Temporal.Instant.prototype.equals(),
    ///     and/or Temporal.Instant.prototype.toString().
    fn value_of<'gc>(
        agent: &mut Agent,
        _: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "`valueOf` not supported by Temporal built-ins. See 'compare', 'equals', or `toString`",
            gc.into_nogc(),
        ))
    }

    // [8.3.15 Temporal.Instant.prototype.toZonedDateTimeISO ( timeZone )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.tozoneddatetimeiso)
    fn to_zoned_date_time_iso<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_instant_prototype();
        let object_prototype = intrinsics.object_prototype();
        let instant_constructor = intrinsics.temporal_instant();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(15)
            .with_prototype(object_prototype)
            .with_constructor_property(instant_constructor)
            .with_builtin_function_property::<TemporalInstantPrototypeGetEpochMilliseconds>()
            .with_builtin_function_property::<TemporalInstantPrototypeGetEpochNanoSeconds>()
            .with_builtin_function_property::<TemporalInstantPrototypeAdd>()
            .with_builtin_function_property::<TemporalInstantPrototypeSubtract>()
            .with_builtin_function_property::<TemporalInstantPrototypeUntil>()
            .with_builtin_function_property::<TemporalInstantPrototypeSince>()
            .with_builtin_function_property::<TemporalInstantPrototypeRound>()
            .with_builtin_function_property::<TemporalInstantPrototypeEquals>()
            .with_builtin_function_property::<TemporalInstantPrototypeToString>()
            .with_builtin_function_property::<TemporalInstantPrototypeToLocaleString>()
            .with_builtin_function_property::<TemporalInstantPrototypeToJSON>()
            .with_builtin_function_property::<TemporalInstantPrototypeValueOf>()
            .with_builtin_function_property::<TemporalInstantPrototypeToZonedDateTimeISO>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal_Instant.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

/// ### [14.5.2.1 GetOptionsObject ( options )](https://tc39.es/proposal-temporal/#sec-getoptionsobject)
///
/// The abstract operation GetOptionsObject takes argument options (an ECMAScript language value)
/// and returns either a normal completion containing an Object or a throw completion. It returns
/// an Object suitable for use with GetOption, either options itself or a default empty Object. It
/// throws a TypeError if options is not undefined and not an Object. It performs the following
/// steps when called:
fn get_options_object<'gc>(
    agent: &mut Agent,
    options: Value,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, Object<'gc>> {
    match options.unbind() {
        // 1. If options is undefined, then
        Value::Undefined => {
            // a. Return OrdinaryObjectCreate(null).
            Ok(ordinary_object_create_with_intrinsics(
                agent, None, None, gc,
            ))
        }
        // 2. If options is an Object, then
        Value::Object(obj) => {
            // a. Return options.
            Ok(obj.into())
        }
        // 3. Throw a TypeError exception.
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "options provided to GetOptionsObject is not an object",
            gc,
        )),
    }
}

trivially_bindable!(RoundingMode);

/// ### [14.5.2.3 GetRoundingModeOption ( options, fallback)](https://tc39.es/proposal-temporal/#sec-temporal-getroundingmodeoption)
///
// The abstract operation GetRoundingModeOption takes arguments options (an Object) and fallback (a
// rounding mode) and returns either a normal completion containing a rounding mode, or a throw
// completion. It fetches and validates the "roundingMode" property from options, returning
// fallback as a default if absent. It performs the following steps when called:
fn get_rounding_mode_option<'gc>(
    _agent: &mut Agent,
    options: Object,
    fallback: RoundingMode,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, RoundingMode> {
    let _options = options.bind(gc.nogc());
    let _fallback = fallback.bind(gc.nogc());
    // 1. Let allowedStrings be the List of Strings from the "String Identifier" column of Table 28.
    // 2. Let stringFallback be the value from the "String Identifier" column of the row with fallback in its "Rounding Mode" column.
    // 3. Let stringValue be ? GetOption(options, "roundingMode", string, allowedStrings, stringFallback).
    // 4. Return the value from the "Rounding Mode" column of the row with stringValue in its "String Identifier" column.
    todo!()
}

trivially_bindable!(RoundingIncrement);

/// ### [14.5.2.4 GetRoundingIncrementOption ( options )](https://tc39.es/proposal-temporal/#sec-temporal-getroundingincrementoption)
///
/// The abstract operation GetRoundingIncrementOption takes argument options (an Object) and returns
/// either a normal completion containing a positive integer in the inclusive interval from 1 to
/// 10**9, or a throw completion. It fetches and validates the "roundingIncrement" property from
/// options, returning a default if absent. It performs the following steps when called:
fn get_rounding_increment_option<'gc>(
    agent: &mut Agent,
    options: Object,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, RoundingIncrement> {
    let options = options.bind(gc.nogc());
    // 1. Let value be ? Get(options, "roundingIncrement").
    let value = get(
        agent,
        options.unbind(),
        BUILTIN_STRING_MEMORY.roundingIncrement.into(),
        gc.reborrow(),
    )
    .unbind()?;
    // 2. If value is undefined, return 1𝔽.
    if value.is_undefined() {
        return Ok(RoundingIncrement::default());
    }
    // 3. Let integerIncrement be ? ToIntegerWithTruncation(value).
    let integer_increment = to_integer_with_truncation(agent, value, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // 4. If integerIncrement < 1 or integerIncrement > 10**9, throw a RangeError exception.
    if integer_increment < 1.0 || integer_increment > 1_000_000_000.0 {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "roundingIncrement must be between 1 and 10**9",
            gc.into_nogc(),
        ));
    }

    // Convert safely and return integerIncrement
    // NOTE: `as u32` is safe here since we validated it’s in range.
    let integer_increment_u32 = integer_increment as u32;
    let increment =
        NonZeroU32::new(integer_increment_u32).expect("integer_increment >= 1 ensures nonzero");

    // 5. Return integerIncrement.
    Ok(RoundingIncrement::new_unchecked(increment))
}

/// ### [13.40 ToIntegerWithTruncation ( argument )] (https://tc39.es/proposal-temporal/#sec-tointegerwithtruncation)
///
/// The abstract operation ToIntegerWithTruncation takes argument argument (an ECMAScript language
/// value) and returns either a normal completion containing an integer or a throw completion. It
/// converts argument to an integer representing its Number value with fractional part truncated, or
/// throws a RangeError when that value is not finite. It performs the following steps when called:
fn to_integer_with_truncation<'gc>(
    agent: &mut Agent,
    argument: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, f64> {
    let argument = argument.bind(gc.nogc());
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // 2. If number is NaN, +∞𝔽 or -∞𝔽, throw a RangeError exception.
    if number.is_nan(agent) || number.is_pos_infinity(agent) || number.is_neg_infinity(agent) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Number cannot be NaN, positive infinity, or negative infinity",
            gc.into_nogc(),
        ));
    }

    // 3. Return truncate(ℝ(number)).
    Ok(number.into_f64(agent).trunc())
}

trivially_bindable!(Unit);

/// ### [13.17 GetTemporalUnitValuedOption ( options, key, default )] (https://tc39.es/proposal-temporal/#sec-temporal-gettemporalunitvaluedoption)
///
/// The abstract operation GetTemporalUnitValuedOption takes arguments options (an Object), key (a
/// property key), and default (required or unset) and returns either a normal completion
/// containing either a Temporal unit, unset, or auto, or a throw completion. It attempts to read a
/// Temporal unit from the specified property of options.
fn get_temporal_unit_valued_option<'gc>(
    _agent: &mut Agent,
    options: Object,
    key: PropertyKey,
    default: DefaultOption,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Unit> {
    let _options = options.bind(gc.nogc());
    let _default = default.bind(gc.nogc());
    todo!()
}

#[allow(dead_code)]
enum DefaultOption {
    Required,
    Unset,
}

trivially_bindable!(DefaultOption);
