use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin,
            temporal::instant::{self, add_duration_to_instant, require_internal_slot_temporal_instant, to_temporal_instant},
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, BigInt, IntoValue, String, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
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
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Let ns be instant.[[EpochNanoseconds]].
        // 4. Let ms be floor(ℝ(ns) / 10**6).
        // 5. Return 𝔽(ms).
        let value = instant.inner_instant(agent).epoch_milliseconds();
        Ok(Value::from_i64(agent, value, gc.into_nogc()))
    }

    /// ### [8.3.4 get Temporal.Instant.prototype.epochNanoseconds](https://tc39.es/proposal-temporal/#sec-get-temporal.instant.prototype.epochnanoseconds)
    fn get_epoch_nanoseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
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
        let result = add_duration_to_instant::<SUBTRACT>(agent, instant.unbind(), duration.unbind(), gc)
            .unbind()?;
        Ok(result.into_value())
    }

    /// ### [8.3.7 Temporal.Instant.prototype.until ( other [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.until)
    fn until<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    /// ### [8.3.8 Temporal.Instant.prototype.since ( other [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.until)
    fn since<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    /// ### [8.3.9 Temporal.Instant.prototype.round ( roundTo )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.round)
    fn round<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    /// ### [8.3.10 Temporal.Instant.prototype.equals ( other )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.equals)
    fn equals<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = require_internal_slot_temporal_instant(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        // 3. Set other to ? ToTemporalInstant(other).
        let other = args.get(0).bind(gc.nogc());
        let other_instant = to_temporal_instant(agent, other.unbind(), gc.reborrow()).unbind()?;
        // 4. If instant.[[EpochNanoseconds]] ≠ other.[[EpochNanoseconds]], return false.
        let instant_val = instant.get(agent).bind(gc.nogc());
        if instant_val.inner_instant(agent) != other_instant {
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
    fn value_of<'gc>(
        agent: &mut Agent,
        _: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Throw a TypeError exception.
        //
        // Note:
        //     This method always throws, because in the absence of valueOf(), expressions with
        //     arithmetic operators such as instant1 > instant2 would fall back to being equivalent
        //     to instant1.toString() > instant2.toString(). Lexicographical comparison of
        //     serialized strings might not seem obviously wrong, because the result would
        //     sometimes be correct. Implementations are encouraged to phrase the error message to
        //     point users to Temporal.Instant.compare(), Temporal.Instant.prototype.equals(),
        //     and/or Temporal.Instant.prototype.toString().
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
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Temporal_Instant.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
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
            .build();
    }
}
