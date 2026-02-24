// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinGetter, JsResult,
        PropertyKey, Realm, String, Value, builders::OrdinaryObjectBuilder,
        builtins::temporal::plain_time::require_internal_slot_temporal_plain_time,
    },
    engine::{GcScope, NoGcScope},
    heap::WellKnownSymbols,
};

pub(crate) struct TemporalPlainTimePrototype;

struct TemporalPlainTimePrototypeGetMinute;
impl Builtin for TemporalPlainTimePrototypeGetMinute {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_minute;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.minute.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalPlainTimePrototype::get_minute);
}
impl BuiltinGetter for TemporalPlainTimePrototypeGetMinute {}

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

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_plain_time_prototype();
        let object_prototype = intrinsics.object_prototype();
        let plain_time_constructor = intrinsics.temporal_plain_time();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(object_prototype)
            .with_constructor_property(plain_time_constructor)
            .with_builtin_function_getter_property::<TemporalPlainTimePrototypeGetMinute>()
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
