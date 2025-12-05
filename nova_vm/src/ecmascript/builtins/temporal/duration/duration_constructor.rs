use std::time::Duration;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_integer_if_integral, builders::builtin_function_builder::BuiltinFunctionBuilder, builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor, temporal::duration::create_temporal_duration}, execution::{Agent, JsResult, Realm, agent::ExceptionType}, types::{BUILTIN_STRING_MEMORY, BigInt, Function, IntoObject, IntoValue, Object, String, Value}
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct TemporalDurationConstructor;

impl Builtin for TemporalDurationConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Duration;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(TemporalDurationConstructor::constructor);
}
impl BuiltinIntrinsicConstructor for TemporalDurationConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalDuration;
}

impl TemporalDurationConstructor {
    /// [7.1.1 Temporal.Duration ( [ years [ , months [ , weeks [ , days [ , hours [ , minutes [ , seconds [ , milliseconds [ , microseconds [ , nanoseconds ] ] ] ] ] ] ] ] ] ] )](https://tc39.es/proposal-temporal/#sec-temporal.duration)
    fn constructor<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let args = args.bind(gc.nogc());
        let new_target = new_target.bind(gc.nogc());

        let years = args.get(1).bind(gc.nogc());
        let months = args.get(2).bind(gc.nogc());
        let weeks: Value<'_> = args.get(3).bind(gc.nogc());
        let days = args.get(4).bind(gc.nogc());
        let hours = args.get(5).bind(gc.nogc());
        let minutes = args.get(6).bind(gc.nogc());
        let seconds = args.get(7).bind(gc.nogc());
        let milliseconds = args.get(8).bind(gc.nogc());
        let microseconds = args.get(9).bind(gc.nogc());
        let nanoseconds = args.get(10).bind(gc.nogc());

        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some (new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin Temporal.Duration constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };
        let Ok(mut new_target) = Function::try_from(new_target) else {
            unreachable!()
        };

        // 2. If years is undefined, let y be 0; else let y be ? ToIntegerIfIntegral(years).
        let y = if years.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, years.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 3. If months is undefined, let mo be 0; else let mo be ? ToIntegerIfIntegral(months).
        let mo = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 4. If weeks is undefined, let w be 0; else let w be ? ToIntegerIfIntegral(weeks).
        let w = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 5. If days is undefined, let d be 0; else let d be ? ToIntegerIfIntegral(days).
        let d = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 6. If hours is undefined, let h be 0; else let h be ? ToIntegerIfIntegral(hours).
        let h = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 7. If minutes is undefined, let m be 0; else let m be ? ToIntegerIfIntegral(minutes).
        let m = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 8. If seconds is undefined, let s be 0; else let s be ? ToIntegerIfIntegral(seconds).
        let s = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 9. If milliseconds is undefined, let ms be 0; else let ms be ? ToIntegerIfIntegral(milliseconds).
        let ms = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_i64(agent)
        };
        // 10. If microseconds is undefined, let mis be 0; else let mis be ? ToIntegerIfIntegral(microseconds).
        let mis = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_f64(agent) as i128
        };
        // 11. If nanoseconds is undefined, let ns be 0; else let ns be ? ToIntegerIfIntegral(nanoseconds).
        let ns = if months.is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.unbind(), gc.reborrow())?.into_f64(agent) as i128
        };
        // 12. Return ? CreateTemporalDuration(y, mo, w, d, h, m, s, ms, mis, ns, NewTarget).
        let duration = temporal_rs::Duration::new(y, mo, w, d, h, m, s, ms, mis, ns)?;
        create_temporal_duration(agent, duration, Some(new_target.unbind()), gc)
            .map(|duration| duration.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _gc: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let duration_prototype = intrinsics.temporal_duration_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TemporalDurationConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype_property(duration_prototype.into_object())
        .build();
    }
}
