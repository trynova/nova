// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, BigInt, Builtin,
        BuiltinIntrinsicConstructor, ExceptionType, Function, InstantRecord, InternalMethods,
        JsResult, Object, Realm, String, Value, builders::BuiltinFunctionBuilder,
        create_temporal_instant, to_big_int, to_temporal_instant,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable},
    heap::{CreateHeapData, IntrinsicConstructorIndexes},
};

/// Constructor function object for %Temporal.Instant%.
pub(crate) struct TemporalInstantConstructor;

impl Builtin for TemporalInstantConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Instant;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(TemporalInstantConstructor::constructor);
}
impl BuiltinIntrinsicConstructor for TemporalInstantConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalInstant;
}

struct TemporalInstantFrom;
impl Builtin for TemporalInstantFrom {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.from;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantConstructor::from);
}

struct TemporalInstantFromEpochMilliseconds;
impl Builtin for TemporalInstantFromEpochMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fromEpochMilliseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(TemporalInstantConstructor::from_epoch_milliseconds);
}

struct TemporalInstantFromEpochNanoseconds;
impl Builtin for TemporalInstantFromEpochNanoseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fromEpochNanoseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(TemporalInstantConstructor::from_epoch_nanoseconds);
}

struct TemporalInstantCompare;
impl Builtin for TemporalInstantCompare {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.compare;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantConstructor::compare);
}

impl TemporalInstantConstructor {
    /// ### [8.1.1 Temporal.Instant ( epochNanoseconds )](https://tc39.es/proposal-temporal/#sec-temporal.instant)
    fn constructor<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let epoch_nanoseconds = args.get(0).bind(gc.nogc());
        let new_target = new_target.bind(gc.nogc());
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin Temporal.Instant constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };
        let Ok(mut new_target) = Function::try_from(new_target) else {
            unreachable!()
        };
        // 2. Let epochNanoseconds be ? ToBigInt(epochNanoseconds).
        let epoch_nanoseconds = if let Ok(epoch_nanoseconds) = BigInt::try_from(epoch_nanoseconds) {
            epoch_nanoseconds
        } else {
            let scoped_new_target = new_target.scope(agent, gc.nogc());
            let epoch_nanoseconds = to_big_int(agent, epoch_nanoseconds.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: not shared.
            new_target = unsafe { scoped_new_target.take(agent) }.bind(gc.nogc());
            epoch_nanoseconds
        };
        // 3. If IsValidEpochNanoseconds(epochNanoseconds) is false, throw a RangeError exception.
        let Some(epoch_nanoseconds) = epoch_nanoseconds
            .try_into_i128(agent)
            .and_then(|nanoseconds| temporal_rs::Instant::try_new(nanoseconds).ok())
        else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "value out of range",
                gc.into_nogc(),
            ));
        };
        // 4. Return ? CreateTemporalInstant(epochNanoseconds, NewTarget).
        create_temporal_instant(agent, epoch_nanoseconds, Some(new_target.unbind()), gc)
            .map(|instant| instant.into())
    }

    /// ### [8.2.2 Temporal.Instant.from ( item )](https://tc39.es/proposal-temporal/#sec-temporal.instant.from)
    fn from<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let item = args.get(0).bind(gc.nogc());
        // 1. Return ? ToTemporalInstant(item).
        let instant = to_temporal_instant(agent, item.unbind(), gc)?;
        let instant = agent.heap.create(InstantRecord {
            object_index: None,
            instant,
        });
        Ok(instant.into())
    }

    /// ### [8.2.3 Temporal.Instant.fromEpochMilliseconds ( epochMilliseconds )](https://tc39.es/proposal-temporal/#sec-temporal.instant.fromepochmilliseconds)
    fn from_epoch_milliseconds<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let epoch_ms = args.get(0).bind(gc.nogc());
        // 1. Set epochMilliseconds to ? ToNumber(epochMilliseconds).
        let epoch_ms_number = epoch_ms
            .unbind()
            .to_number(agent, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 2. Set epochMilliseconds to ? NumberToBigInt(epochMilliseconds).
        if !epoch_ms_number.is_integer(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Can't convert number to BigInt because it isn't an integer",
                gc.into_nogc(),
            ));
        }
        // 3. Let epochNanoseconds be epochMilliseconds × ℤ(10**6).
        // 4. If IsValidEpochNanoseconds(epochNanoseconds) is false, throw a RangeError exception.
        let epoch_ns =
            match temporal_rs::Instant::from_epoch_milliseconds(epoch_ms_number.into_i64(agent)) {
                Ok(instant) => instant,
                Err(_) => {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "epochMilliseconds value out of range",
                        gc.into_nogc(),
                    ));
                }
            };

        // 5. Return ! CreateTemporalInstant(epochNanoseconds).
        let instant = create_temporal_instant(agent, epoch_ns, None, gc)?;
        let value = instant.into();
        Ok(value)
    }

    /// ### [8.2.4 Temporal.Instant.fromEpochNanoseconds ( epochNanoseconds )] (https://tc39.es/proposal-temporal/#sec-temporal.instant.fromepochnanoseconds)
    fn from_epoch_nanoseconds<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let epoch_nanoseconds = arguments.get(0).bind(gc.nogc());
        // 1. Set epochNanoseconds to ? ToBigInt(epochNanoseconds).
        let epoch_nanoseconds = if let Ok(epoch_nanoseconds) = BigInt::try_from(epoch_nanoseconds) {
            epoch_nanoseconds
        } else {
            to_big_int(agent, epoch_nanoseconds.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };
        // 2. If IsValidEpochNanoseconds(epochNanoseconds) is false, throw a RangeError exception.
        let Some(epoch_nanoseconds) = epoch_nanoseconds
            .try_into_i128(agent)
            .and_then(|nanoseconds| temporal_rs::Instant::try_new(nanoseconds).ok())
        else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "epochNanoseconds",
                gc.into_nogc(),
            ));
        };
        // 3. Return ! CreateTemporalInstant(epochNanoseconds).
        let instant = create_temporal_instant(agent, epoch_nanoseconds, None, gc)?;
        let value = instant.into();
        Ok(value)
    }

    /// ### [8.2.5 Temporal.Instant.compare ( one, two )](https://tc39.es/proposal-temporal/#sec-temporal.instant.compare)
    fn compare<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let one = args.get(0).bind(gc.nogc());
        let two = args.get(1).bind(gc.nogc());

        let res = if let (Value::Instant(one), Value::Instant(two)) = (one, two) {
            one.inner_instant(agent).cmp(two.inner_instant(agent))
        } else {
            // TODO(jesper): in the case of one being an instant and not the other, only create one temporal_rs::Instant
            let two = two.scope(agent, gc.nogc());
            // 1. Set one to ? ToTemporalInstant(one).
            let one_instant = to_temporal_instant(agent, one.unbind(), gc.reborrow()).unbind()?;
            // 2. Set two to ? ToTemporalInstant(two).
            let two_value = two.get(agent).bind(gc.nogc());
            let two_instant =
                to_temporal_instant(agent, two_value.unbind(), gc.reborrow()).unbind()?;

            one_instant.cmp(&two_instant)
        };

        // 3. Return 𝔽(CompareEpochNanoseconds(one.[[EpochNanoseconds]], two.[[EpochNanoseconds]])).
        Ok((res as i8).into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _gc: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let instant_prototype = intrinsics.temporal_instant_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TemporalInstantConstructor>(
            agent, realm,
        )
        .with_property_capacity(5)
        .with_prototype_property(instant_prototype.into())
        .with_builtin_function_property::<TemporalInstantFrom>()
        .with_builtin_function_property::<TemporalInstantFromEpochMilliseconds>()
        .with_builtin_function_property::<TemporalInstantFromEpochNanoseconds>()
        .with_builtin_function_property::<TemporalInstantCompare>()
        .build();
    }
}
