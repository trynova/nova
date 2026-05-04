// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use temporal_rs::options::ToStringRoundingOptions;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter,
        ExceptionType, JsResult, PropertyKey, Realm, String, Value,
        builders::OrdinaryObjectBuilder,
        builtins::temporal::plain_time::{
            add_duration_to_time, require_internal_slot_temporal_plain_time,
        },
        temporal_err_to_js_err,
    },
    engine::{Bindable, GcScope, NoGcScope},
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

struct TemporalPlainTimePrototypeToJSON;
impl Builtin for TemporalPlainTimePrototypeToJSON {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toJSON;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::to_json);
}

struct TemporalPlainTimePrototypeValueOf;
impl Builtin for TemporalPlainTimePrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::value_of);
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

    /// ### [4.3.18 Temporal.PlainTime.prototype.toJSON ( )](https://tc39.es/proposal-temporal/#sec-temporal.plaintime.prototype.tojson)
    fn to_json<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // 1. Let plainTime be the this value.
        let value = this_value.bind(gc);
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let instant = require_internal_slot_temporal_plain_time(agent, value.unbind(), gc)
            .unbind()?
            .bind(gc);
        // 3. Return TimeRecordToString(plainTime.[[Time]], auto).
        let options: ToStringRoundingOptions = ToStringRoundingOptions::default();
        match instant.inner_plain_time(agent).to_ixdtf_string(options) {
            Ok(string) => Ok(Value::from_string(agent, string, gc.into_nogc())),
            Err(err) => Err(temporal_err_to_js_err(agent, err, gc.into_nogc())),
        }
    }

    /// ### [4.3.19 Temporal.PlainTime.prototype.valueOf](https://tc39.es/proposal-temporal/#sec-temporal.plaintime.prototype.valueof)
    fn value_of<'gc>(
        agent: &mut Agent,
        _: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "can't convert PlainTime to primitive type. Use PlainTime.prototype.equals() or PlainTime.compare() instead.",
            gc.into_nogc(),
        ))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_plain_time_prototype();
        let object_prototype = intrinsics.object_prototype();
        let plain_time_constructor = intrinsics.temporal_plain_time();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(12)
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
            .with_builtin_function_property::<TemporalPlainTimePrototypeToJSON>()
            .with_builtin_function_property::<TemporalPlainTimePrototypeValueOf>()
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
