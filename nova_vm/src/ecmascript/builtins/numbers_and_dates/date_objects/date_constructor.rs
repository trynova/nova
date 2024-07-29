// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::SystemTime;

use crate::ecmascript::abstract_operations::type_conversion::to_number;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::date::Date;
use crate::ecmascript::builtins::ordinary::ordinary_create_from_constructor;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Number;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::{String, Value};
use crate::heap::IntrinsicConstructorIndexes;
use crate::SmallInteger;

pub struct DateConstructor;

impl Builtin for DateConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 7;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Date;
}
impl BuiltinIntrinsicConstructor for DateConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Date;
}

struct DateNow;
impl Builtin for DateNow {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DateConstructor::now);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.now;
}
struct DateParse;
impl Builtin for DateParse {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DateConstructor::parse);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.parse;
}
struct DateUTC;
impl Builtin for DateUTC {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DateConstructor::utc);
    const LENGTH: u8 = 7;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.utc;
}
impl DateConstructor {
    fn behaviour<'gen>(
        agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        arguments: ArgumentsList<'_, 'gen>,
        new_target: Option<Object<'gen>>,
    ) -> JsResult<'gen, Value<'gen>> {
        // 1. If NewTarget is undefined, then
        let Some(new_target) = new_target else {
            // a. Let now be the time value (UTC) identifying the current time.
            let _now = SystemTime::now();
            // b. Return ToDateString(now).
            todo!("ToDateString(now)");
        };
        // 2. Let numberOfArgs be the number of elements in values.
        let number_of_args = arguments.len() as u32;
        let dv = match number_of_args {
            // 3. If numberOfArgs = 0, then
            0 => {
                // a. Let dv be the time value (UTC) identifying the current time.
                SystemTime::now()
            }
            // 4. Else if numberOfArgs = 1, then
            1 => {
                todo!();
                // a. Let value be values[0].
                // b. If value is an Object and value has a [[DateValue]] internal slot, then
                // i. Let tv be value.[[DateValue]].
                // c. Else,
                // i. Let v be ? ToPrimitive(value).
                // ii. If v is a String, then
                // 1. Assert: The next step never returns an abrupt completion because v is a String.
                // 2. Let tv be the result of parsing v as a date, in exactly the same manner as for the parse method (21.4.3.2).
                // iii. Else,
                // 1. Let tv be ? ToNumber(v).
                // d. Let dv be TimeClip(tv).
            }
            // 5. Else,
            _ => {
                todo!();
                // a. Assert: numberOfArgs ≥ 2.
                // b. Let y be ? ToNumber(values[0]).
                // c. Let m be ? ToNumber(values[1]).
                // d. If numberOfArgs > 2, let dt be ? ToNumber(values[2]); else let dt be 1𝔽.
                // e. If numberOfArgs > 3, let h be ? ToNumber(values[3]); else let h be +0𝔽.
                // f. If numberOfArgs > 4, let min be ? ToNumber(values[4]); else let min be +0𝔽.
                // g. If numberOfArgs > 5, let s be ? ToNumber(values[5]); else let s be +0𝔽.
                // h. If numberOfArgs > 6, let milli be ? ToNumber(values[6]); else let milli be +0𝔽.
                // i. Let yr be MakeFullYear(y).
                // j. Let finalDate be MakeDate(MakeDay(yr, m, dt), MakeTime(h, min, s, milli)).
                // k. Let dv be TimeClip(UTC(finalDate)).
            }
        };

        // 6. Let O be ? OrdinaryCreateFromConstructor(NewTarget, "%Date.prototype%", « [[DateValue]] »).
        let o = ordinary_create_from_constructor(
            agent,
            Function::try_from(new_target).unwrap(),
            ProtoIntrinsics::Date,
        )?;
        // 7. Set O.[[DateValue]] to dv.
        agent[Date::try_from(o).unwrap()].date = Some(dv);
        // 8. Return O.
        Ok(o.into_value())
    }

    /// ### [21.1.2.2 Number.isFinite ( number )](https://tc39.es/ecma262/#sec-number.isfinite)
    fn now<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        let time_value = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        assert_eq!(time_value as u64 as u128, time_value);
        let time_value = time_value as u64;
        Ok(
            Number::from(SmallInteger::try_from(time_value).expect("SystemTime is beyond range"))
                .into_value(),
        )
    }

    /// ### [21.1.2.3 Number.isInteger ( number )](https://tc39.es/ecma262/#sec-number.isinteger)
    fn parse<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    /// ### [21.4.3.4 Date.UTC ( year \[ , month \[ , date \[ , hours \[ , minutes \[ , seconds \[ , ms \] \] \] \] \] \] )](https://tc39.es/ecma262/#sec-date.utc)
    fn utc<'gen>(agent: &mut Agent<'gen>, _this_value: Value<'gen>, arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        let _ns = arguments.get(0);
        // 1. Let y be ? ToNumber(year).
        let _y = to_number(agent, arguments.get(0))?;
        // 2. If month is present, let m be ? ToNumber(month); else let m be +0𝔽.
        let _m = if arguments.len() > 1 {
            to_number(agent, arguments.get(1))?
        } else {
            0.into()
        };
        // 3. If date is present, let dt be ? ToNumber(date); else let dt be 1𝔽.
        let _dt = if arguments.len() > 2 {
            to_number(agent, arguments.get(2))?
        } else {
            0.into()
        };
        // 4. If hours is present, let h be ? ToNumber(hours); else let h be +0𝔽.
        let _h = if arguments.len() > 3 {
            to_number(agent, arguments.get(3))?
        } else {
            0.into()
        };
        // 5. If minutes is present, let min be ? ToNumber(minutes); else let min be +0𝔽.
        let _min = if arguments.len() > 4 {
            to_number(agent, arguments.get(4))?
        } else {
            0.into()
        };
        // 6. If seconds is present, let s be ? ToNumber(seconds); else let s be +0𝔽.
        let _s = if arguments.len() > 5 {
            to_number(agent, arguments.get(5))?
        } else {
            0.into()
        };
        // 7. If ms is present, let milli be ? ToNumber(ms); else let milli be +0𝔽.
        let _milli = if arguments.len() > 6 {
            to_number(agent, arguments.get(6))?
        } else {
            0.into()
        };
        // 8. Let yr be MakeFullYear(y).
        todo!("MakeFullYear");
        // 9. Return TimeClip(MakeDate(MakeDay(yr, m, dt), MakeTime(h, min, s, milli))).

        // Note
        // This function differs from the Date constructor in two ways: it
        // returns a time value as a Number, rather than creating a Date,
        // and it interprets the arguments in UTC rather than as local time.
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let date_prototype = intrinsics.date_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<DateConstructor>(agent, realm)
            .with_property_capacity(4)
            .with_builtin_function_property::<DateNow>()
            .with_builtin_function_property::<DateParse>()
            .with_prototype_property(date_prototype.into_object())
            .with_builtin_function_property::<DateUTC>()
            .build();
    }
}
