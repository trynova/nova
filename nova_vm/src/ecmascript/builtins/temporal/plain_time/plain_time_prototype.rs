// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter, JsResult,
        PropertyKey, Realm, String, Value, builders::OrdinaryObjectBuilder,
        builtins::temporal::plain_time::{self, require_internal_slot_temporal_plain_time},
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

struct TemporalPlainTimePrototypeUntil;
impl Builtin for TemporalPlainTimePrototypeUntil {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.until;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::until);
}

struct TemporalPlainTimePrototypeSince;
impl Builtin for TemporalPlainTimePrototypeSince {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.since;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::since);
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

    /// ### [4.3.12 Temporal.PlainTime.prototype.until ( other [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal.plaintime.prototype.until) 
    fn until<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>>{
        let other = args.get(0).bind(gc.nogc());
        let options = args.get(1).bind(gc.nogc());
        // 1. Let plainTime be the this value.
        let plain_time = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, plain_time.unbind(), gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Return ? DifferenceTemporalPlainTime(until, plainTime, other, options).
        const UNTIL: bool = true;
        difference_temporal_plain_time::<UNTIL>(
            agent,
            plain_time.unbind(),
            other.unbind(),
            options.unbind(),
            gc,
        )
        .map(Value::from)
    }

    /// ### [4.3.13 Temporal.PlainTime.prototype.since ( other [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal.plaintime.prototype.since)
    fn since<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let other = args.get(0).bind(gc.nogc());
        let options = args.get(1).bind(gc.nogc());
        // 1. Let plainTime be the this value.
        let plain_time = this_value.bind(gc.nogc());
        // 2. Perform ? RequireInternalSlot(plainTime, [[InitializedTemporalTime]]).
        let plain_time = require_internal_slot_temporal_plain_time(agent, plain_time.unbind(), gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Return ? DifferenceTemporalPlainTime(since, instant, other, options).
        const SINCE: bool = false;
        difference_temporal_plain_time::<SINCE>(
            agent,
            plain_time.unbind(),
            other.unbind(),
            options.unbind(),
            gc,
        )
        .map(Value::from)
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
            .with_builtin_function_property::<TemporalPlainTimePrototypeSince>()
            .with_builtin_function_property::<TemporalPlainTimePrototypeUntil>()
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
