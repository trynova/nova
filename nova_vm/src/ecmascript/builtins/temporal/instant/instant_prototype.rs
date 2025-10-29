use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin,
            temporal::instant::requrire_temporal_instant_internal_slot,
        },
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, BigInt, String, Value},
    },
    engine::context::{Bindable, GcScope, NoGcScope},
};

/// %Temporal.Instant.Prototype%
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

impl TemporalInstantPrototype {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_instant_prototype();
        let object_prototype = intrinsics.object_prototype();
        let instant_constructor = intrinsics.temporal_instant();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(7)
            .with_prototype(object_prototype)
            .with_constructor_property(instant_constructor)
            .with_builtin_function_property::<TemporalInstantPrototypeGetEpochMilliseconds>()
            .with_builtin_function_property::<TemporalInstantPrototypeGetEpochNanoSeconds>()
            .with_builtin_function_property::<TemporalInstantPrototypeAdd>()
            .with_builtin_function_property::<TemporalInstantPrototypeSubtract>()
            .with_builtin_function_property::<TemporalInstantPrototypeUntil>()
            .with_builtin_function_property::<TemporalInstantPrototypeSince>()
            .build();
    }

    /// ### [8.3.3 get Temporal.Instant.prototype.epochMilliseconds](https://tc39.es/proposal-temporal/#sec-get-temporal.instant.prototype.epochmilliseconds)
    fn get_epoch_milliseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let instant be the this value.
        // 2. Perform ? RequireInternalSlot(instant, [[InitializedTemporalInstant]]).
        let instant = requrire_temporal_instant_internal_slot(agent, this_value, gc.nogc())
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
        let instant = requrire_temporal_instant_internal_slot(agent, this_value, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
        // 3. Return instant.[[EpochNanoseconds]].
        let value = instant.inner_instant(agent).epoch_nanoseconds().as_i128();
        Ok(BigInt::from_i128(agent, value).into())
    }

    /// ### [8.3.5 Temporal.Instant.prototype.add ( temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.add)
    fn add<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    /// ### [8.3.6 Temporal.Instant.prototype.subtract ( temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.subtract)
    fn subtract<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
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

    /// ### [Temporal.Instant.prototype.since ( other [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal.instant.prototype.until)
    fn since<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _args: ArgumentsList,
        mut _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }
}
