// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::SystemTime;

use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{ordinary_to_primitive, PreferredType},
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{date::Date, ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, Object, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
    SmallInteger,
};

pub(crate) struct DatePrototype;

struct DatePrototypeGetDate;
impl Builtin for DatePrototypeGetDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getDate;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_date);
}
struct DatePrototypeGetDay;
impl Builtin for DatePrototypeGetDay {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getDay;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_day);
}
struct DatePrototypeGetFullYear;
impl Builtin for DatePrototypeGetFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_full_year);
}
struct DatePrototypeGetHours;
impl Builtin for DatePrototypeGetHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_hours);
}
struct DatePrototypeGetMilliseconds;
impl Builtin for DatePrototypeGetMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_milliseconds);
}
struct DatePrototypeGetMinutes;
impl Builtin for DatePrototypeGetMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_minutes);
}
struct DatePrototypeGetMonth;
impl Builtin for DatePrototypeGetMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_month);
}
struct DatePrototypeGetSeconds;
impl Builtin for DatePrototypeGetSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_seconds);
}
struct DatePrototypeGetTime;
impl Builtin for DatePrototypeGetTime {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getTime;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_time);
}
struct DatePrototypeGetTimezoneOffset;
impl Builtin for DatePrototypeGetTimezoneOffset {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getTimezoneOffset;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_timezone_offset);
}
struct DatePrototypeGetUTCDate;
impl Builtin for DatePrototypeGetUTCDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCDate;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_date);
}
struct DatePrototypeGetUTCDay;
impl Builtin for DatePrototypeGetUTCDay {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCDay;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_day);
}
struct DatePrototypeGetUTCFullYear;
impl Builtin for DatePrototypeGetUTCFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_full_year);
}
struct DatePrototypeGetUTCHours;
impl Builtin for DatePrototypeGetUTCHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_hours);
}
struct DatePrototypeGetUTCMilliseconds;
impl Builtin for DatePrototypeGetUTCMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_milliseconds);
}
struct DatePrototypeGetUTCMinutes;
impl Builtin for DatePrototypeGetUTCMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_minutes);
}
struct DatePrototypeGetUTCMonth;
impl Builtin for DatePrototypeGetUTCMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_month);
}
struct DatePrototypeGetUTCSeconds;
impl Builtin for DatePrototypeGetUTCSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_seconds);
}
struct DatePrototypeSetDate;
impl Builtin for DatePrototypeSetDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setDate;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_date);
}
struct DatePrototypeSetFullYear;
impl Builtin for DatePrototypeSetFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setFullYear;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_full_year);
}
struct DatePrototypeSetHours;
impl Builtin for DatePrototypeSetHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setHours;
    const LENGTH: u8 = 4;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_hours);
}
struct DatePrototypeSetMilliseconds;
impl Builtin for DatePrototypeSetMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setMilliseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_milliseconds);
}
struct DatePrototypeSetMinutes;
impl Builtin for DatePrototypeSetMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setMinutes;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_minutes);
}
struct DatePrototypeSetMonth;
impl Builtin for DatePrototypeSetMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setMonth;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_month);
}
struct DatePrototypeSetSeconds;
impl Builtin for DatePrototypeSetSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setSeconds;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_seconds);
}
struct DatePrototypeSetTime;
impl Builtin for DatePrototypeSetTime {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setTime;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_time);
}
struct DatePrototypeSetUTCDate;
impl Builtin for DatePrototypeSetUTCDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCDate;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_date);
}
struct DatePrototypeSetUTCFullYear;
impl Builtin for DatePrototypeSetUTCFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCFullYear;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_full_year);
}
struct DatePrototypeSetUTCHours;
impl Builtin for DatePrototypeSetUTCHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCHours;
    const LENGTH: u8 = 4;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_hours);
}
struct DatePrototypeSetUTCMilliseconds;
impl Builtin for DatePrototypeSetUTCMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCMilliseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_milliseconds);
}
struct DatePrototypeSetUTCMinutes;
impl Builtin for DatePrototypeSetUTCMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCMinutes;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_minutes);
}
struct DatePrototypeSetUTCMonth;
impl Builtin for DatePrototypeSetUTCMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCMonth;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_month);
}
struct DatePrototypeSetUTCSeconds;
impl Builtin for DatePrototypeSetUTCSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCSeconds;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_seconds);
}
struct DatePrototypeToDateString;
impl Builtin for DatePrototypeToDateString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toDateString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_date_string);
}
struct DatePrototypeToISOString;
impl Builtin for DatePrototypeToISOString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toISOString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_iso_string);
}
struct DatePrototypeToJSON;
impl Builtin for DatePrototypeToJSON {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toJSON;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_json);
}
struct DatePrototypeToLocaleDateString;
impl Builtin for DatePrototypeToLocaleDateString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleDateString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_locale_date_string);
}
struct DatePrototypeToLocaleString;
impl Builtin for DatePrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_locale_string);
}
struct DatePrototypeToLocaleTimeString;
impl Builtin for DatePrototypeToLocaleTimeString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleTimeString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_locale_time_string);
}
struct DatePrototypeToString;
impl Builtin for DatePrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_string);
}
struct DatePrototypeToTimeString;
impl Builtin for DatePrototypeToTimeString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toTimeString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_time_string);
}
struct DatePrototypeToUTCString;
impl Builtin for DatePrototypeToUTCString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toUTCString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_utc_string);
}
impl BuiltinIntrinsic for DatePrototypeToUTCString {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::DatePrototypeToUTCString;
}
struct DatePrototypeValueOf;
impl Builtin for DatePrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::value_of);
}
struct DatePrototypeToPrimitive;
impl Builtin for DatePrototypeToPrimitive {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_toPrimitive_;

    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::ToPrimitive.to_property_key());

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(DatePrototype::to_primitive);

    const WRITABLE: bool = false;
}

const MAX_SYSTEM_TIME_VALUE: u128 = SmallInteger::MAX_NUMBER as u128;

impl<'gc> DatePrototype {
    fn get_date(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_day(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_full_year(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_hours(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_milliseconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_minutes(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_month(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_seconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_time(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_timezone_offset(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_date(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_day(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_full_year(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_hours(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_milliseconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_minutes(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_month(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_seconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_date(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_full_year(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_hours(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_milliseconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_minutes(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_month(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_seconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_time(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_date(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_full_year(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_hours(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_milliseconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_minutes(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_month(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_seconds(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_date_string(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_iso_string(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_json(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_locale_date_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_locale_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_locale_time_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_string(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_time_string(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_utc_string(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = check_date_object(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn value_of(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let date_object = check_date_object(agent, this_value, gc.nogc())?;
        let data = &agent[date_object].date;
        match data {
            Some(system_time) => {
                let time_as_millis = system_time
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map_or_else(
                        |_| {
                            // System time is before UNIX_EPOCH
                            let value = SystemTime::UNIX_EPOCH
                                .duration_since(*system_time)
                                .unwrap()
                                .as_millis();
                            if value > MAX_SYSTEM_TIME_VALUE {
                                // Time difference is over representable limit
                                None
                            } else {
                                Some(-(value as i64))
                            }
                        },
                        |value| {
                            let value = value.as_millis();
                            if value > MAX_SYSTEM_TIME_VALUE {
                                None
                            } else {
                                Some(value as i64)
                            }
                        },
                    );
                match time_as_millis {
                    Some(time_as_millis) => Ok(Number::from(
                        SmallInteger::try_from(time_as_millis).unwrap(),
                    )
                    .into_value()),
                    None => Ok(Value::nan()),
                }
            }
            None => Ok(Value::nan()),
        }
    }

    /// ### [21.4.4.45 Date.prototype \[ %Symbol.toPrimitive% \] ( hint )](https://tc39.es/ecma262/#sec-date.prototype-%symbol.toprimitive%)
    ///
    /// This method is called by ECMAScript language operators to convert a
    /// Date to a primitive value. The allowed values for hint are "default",
    /// "number", and "string". Dates are unique among built-in ECMAScript
    /// object in that they treat "default" as being equivalent to "string".
    /// All other built-in ECMAScript objects treat "default" as being
    /// equivalent to "number".
    fn to_primitive(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let hint = arguments.get(0);
        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            let error_message = format!(
                "{} is not an object",
                this_value.string_repr(agent, gc.reborrow()).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc()));
        };
        // 3. If hint is either "string" or "default", then
        let try_first = if hint == BUILTIN_STRING_MEMORY.string.into_value()
            || hint == BUILTIN_STRING_MEMORY.default.into_value()
        {
            // a. Let tryFirst be string.
            PreferredType::String
        } else if hint == BUILTIN_STRING_MEMORY.number.into_value() {
            // 4. Else if hint is "number", then
            // a. Let tryFirst be number.
            PreferredType::Number
        } else {
            // 5. Else,
            // a. Throw a TypeError exception.
            let error_message = format!(
                "Expected 'hint' to be \"string\", \"default\", or \"number\", got {}",
                hint.string_repr(agent, gc.reborrow()).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc()));
        };
        // 6. Return ? OrdinaryToPrimitive(O, tryFirst).
        ordinary_to_primitive(agent, o, try_first, gc.reborrow()).map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.date_prototype();
        let date_constructor = intrinsics.date();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(45)
            .with_prototype(object_prototype)
            .with_constructor_property(date_constructor)
            .with_builtin_function_property::<DatePrototypeGetDate>()
            .with_builtin_function_property::<DatePrototypeGetDay>()
            .with_builtin_function_property::<DatePrototypeGetFullYear>()
            .with_builtin_function_property::<DatePrototypeGetHours>()
            .with_builtin_function_property::<DatePrototypeGetMilliseconds>()
            .with_builtin_function_property::<DatePrototypeGetMinutes>()
            .with_builtin_function_property::<DatePrototypeGetMonth>()
            .with_builtin_function_property::<DatePrototypeGetSeconds>()
            .with_builtin_function_property::<DatePrototypeGetTime>()
            .with_builtin_function_property::<DatePrototypeGetTimezoneOffset>()
            .with_builtin_function_property::<DatePrototypeGetUTCDate>()
            .with_builtin_function_property::<DatePrototypeGetUTCDay>()
            .with_builtin_function_property::<DatePrototypeGetUTCFullYear>()
            .with_builtin_function_property::<DatePrototypeGetUTCHours>()
            .with_builtin_function_property::<DatePrototypeGetUTCMilliseconds>()
            .with_builtin_function_property::<DatePrototypeGetUTCMinutes>()
            .with_builtin_function_property::<DatePrototypeGetUTCMonth>()
            .with_builtin_function_property::<DatePrototypeGetUTCSeconds>()
            .with_builtin_function_property::<DatePrototypeSetDate>()
            .with_builtin_function_property::<DatePrototypeSetFullYear>()
            .with_builtin_function_property::<DatePrototypeSetHours>()
            .with_builtin_function_property::<DatePrototypeSetMilliseconds>()
            .with_builtin_function_property::<DatePrototypeSetMinutes>()
            .with_builtin_function_property::<DatePrototypeSetMonth>()
            .with_builtin_function_property::<DatePrototypeSetSeconds>()
            .with_builtin_function_property::<DatePrototypeSetTime>()
            .with_builtin_function_property::<DatePrototypeSetUTCDate>()
            .with_builtin_function_property::<DatePrototypeSetUTCFullYear>()
            .with_builtin_function_property::<DatePrototypeSetUTCHours>()
            .with_builtin_function_property::<DatePrototypeSetUTCMilliseconds>()
            .with_builtin_function_property::<DatePrototypeSetUTCMinutes>()
            .with_builtin_function_property::<DatePrototypeSetUTCMonth>()
            .with_builtin_function_property::<DatePrototypeSetUTCSeconds>()
            .with_builtin_function_property::<DatePrototypeToDateString>()
            .with_builtin_function_property::<DatePrototypeToISOString>()
            .with_builtin_function_property::<DatePrototypeToJSON>()
            .with_builtin_function_property::<DatePrototypeToLocaleDateString>()
            .with_builtin_function_property::<DatePrototypeToLocaleString>()
            .with_builtin_function_property::<DatePrototypeToLocaleTimeString>()
            .with_builtin_function_property::<DatePrototypeToString>()
            .with_builtin_function_property::<DatePrototypeToTimeString>()
            .with_builtin_intrinsic_function_property::<DatePrototypeToUTCString>()
            .with_builtin_function_property::<DatePrototypeValueOf>()
            .with_builtin_function_property::<DatePrototypeToPrimitive>()
            .build();
    }
}

#[inline(always)]
fn check_date_object<'a>(
    agent: &mut Agent,
    this_value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<Date<'a>> {
    match this_value {
        Value::Date(date) => Ok(date),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "this is not a Date object.",
            gc,
        )),
    }
}
