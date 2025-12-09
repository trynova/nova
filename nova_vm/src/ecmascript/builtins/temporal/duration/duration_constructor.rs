use std::{thread::scope, time::Duration};

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_integer_if_integral,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
            temporal::{duration::create_temporal_duration, error::temporal_err_to_js_err},
        },
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, BigInt, Function, IntoObject, IntoValue, Object, String, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
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
                "calling a builtin Temporal.Duration constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };
        let Ok(new_target) = Function::try_from(new_target.unbind()) else {
            unreachable!()
        };
        let new_target = new_target.scope(agent, gc.nogc());
        // 2. If years is undefined, let y be 0; else let y be ? ToIntegerIfIntegral(years).
        let y = if years.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, years.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 3. If months is undefined, let mo be 0; else let mo be ? ToIntegerIfIntegral(months).
        let mo = if months.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, months.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 4. If weeks is undefined, let w be 0; else let w be ? ToIntegerIfIntegral(weeks).
        let w = if weeks.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, weeks.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 5. If days is undefined, let d be 0; else let d be ? ToIntegerIfIntegral(days).
        let d = if days.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, days.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 6. If hours is undefined, let h be 0; else let h be ? ToIntegerIfIntegral(hours).
        let h = if hours.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, hours.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 7. If minutes is undefined, let m be 0; else let m be ? ToIntegerIfIntegral(minutes).
        let m = if minutes.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, minutes.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 8. If seconds is undefined, let s be 0; else let s be ? ToIntegerIfIntegral(seconds).
        let s = if seconds.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, seconds.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 9. If milliseconds is undefined, let ms be 0; else let ms be ? ToIntegerIfIntegral(milliseconds).
        let ms = if milliseconds.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, milliseconds.get(agent), gc.reborrow())
                .unbind()?
                .into_i64(agent)
        }
        .bind(gc.nogc());

        // 10. If microseconds is undefined, let mis be 0; else let mis be ? ToIntegerIfIntegral(microseconds).
        let mis = if microseconds.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, microseconds.get(agent), gc.reborrow())
                .unbind()?
                .into_f64(agent) as i128
        }
        .bind(gc.nogc());

        // 11. If nanoseconds is undefined, let ns be 0; else let ns be ? ToIntegerIfIntegral(nanoseconds).
        let ns = if nanoseconds.get(agent).is_undefined() {
            0
        } else {
            to_integer_if_integral(agent, nanoseconds.get(agent), gc.reborrow())
                .unbind()?
                .into_f64(agent) as i128
        }
        .bind(gc.nogc());
        // 12. Return ? CreateTemporalDuration(y, mo, w, d, h, m, s, ms, mis, ns, NewTarget).
        let duration = temporal_rs::Duration::new(y, mo, w, d, h, m, s, ms, mis, ns)
            .map_err(|e| temporal_err_to_js_err(agent, e, gc.nogc()))
            .unbind()?
            .bind(gc.nogc());
        create_temporal_duration(agent, duration.unbind(), Some(new_target.get(agent)), gc)
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
