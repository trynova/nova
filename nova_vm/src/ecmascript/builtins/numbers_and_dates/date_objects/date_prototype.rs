// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::SystemTime;

use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::type_conversion::{PreferredType, ordinary_to_primitive},
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic, date::Date},
        execution::{Agent, JsResult, RealmIdentifier, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, Number, Object, PropertyKey, String, Value},
    },
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
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

    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_primitive);

    const WRITABLE: bool = false;
}

const MAX_SYSTEM_TIME_VALUE: u128 = SmallInteger::MAX_NUMBER as u128;

impl DatePrototype {
    /// ### [21.4.4.2 Date.prototype.getDate ( )](https://tc39.es/ecma262/#sec-date.prototype.getdate)
    ///
    /// This method performs the following steps when called:
    fn get_date<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t else {
            return Ok(Value::nan());
        };
        // 5. Return DateFromTime(LocalTime(t)).
        Ok(Value::Integer(date_from_time(local_time(agent, t)).into()))
    }

    /// ### [21.4.4.3 Date.prototype.getDay ( )](https://tc39.es/ecma262/#sec-date.prototype.getday)
    ///
    /// This method performs the following steps when called:
    fn get_day<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t else {
            return Ok(Value::nan());
        };
        // 5. Return WeekDay(LocalTime(t)).
        Ok(Value::Integer(week_day(local_time(agent, t)).into()))
    }

    /// ### [21.4.4.4 Date.prototype.getFullYear ( )](https://tc39.es/ecma262/#sec-date.prototype.getfullyear)
    ///
    /// This method performs the following steps when called:
    fn get_full_year<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t else {
            return Ok(Value::nan());
        };
        // 5. Return YearFromTime(LocalTime(t)).
        Ok(Value::Integer(year_from_time(local_time(agent, t)).into()))
    }

    fn get_hours<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_milliseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_minutes<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_month<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_seconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_time<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_timezone_offset<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_date<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_day<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_full_year<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_hours<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_milliseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_minutes<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_month<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn get_utc_seconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_date<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_full_year<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_hours<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_milliseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_minutes<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_month<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_seconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_time<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_date<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_full_year<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_hours<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_milliseconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_minutes<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_month<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn set_utc_seconds<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_date_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_iso_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_json<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_locale_date_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_locale_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_locale_time_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_time_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn to_utc_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let _date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        todo!()
    }

    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
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
    fn to_primitive<'gc>(
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
        Ok(ordinary_to_primitive(agent, o, try_first, gc.reborrow())?
            .into_value()
            .unbind()
            .bind(gc.into_nogc()))
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
fn require_internal_slot_date<'a>(
    agent: &mut Agent,
    this_value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<Date<'a>> {
    match this_value {
        Value::Date(date) => Ok(date.bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "this is not a Date object.",
            gc,
        )),
    }
}

/// ### [21.4.1.2 Time-related Constants](https://tc39.es/ecma262/#sec-time-related-constants)
/// These constants are referenced by algorithms in the following sections.
/// HoursPerDay = 24
const HOURS_PER_DAY: f64 = 24.0;
/// MinutesPerHour = 60
const MINUTES_PER_HOUR: f64 = 60.0;
/// SecondsPerMinute = 60
const SECONDS_PER_MINUTE: f64 = 60.0;
/// msPerSecond = 1000𝔽
const MS_PER_SECOND: f64 = 1000.0;
/// msPerMinute = 60000𝔽 = msPerSecond × 𝔽(SecondsPerMinute)
const MS_PER_MINUTE: f64 = MS_PER_SECOND * SECONDS_PER_MINUTE;
/// msPerHour = 3600000𝔽 = msPerMinute × 𝔽(MinutesPerHour)
const MS_PER_HOUR: f64 = MS_PER_MINUTE * MINUTES_PER_HOUR;
/// msPerDay = 86400000𝔽 = msPerHour × 𝔽(HoursPerDay)
const MS_PER_DAY: f64 = MS_PER_HOUR * HOURS_PER_DAY;

/// ### [21.4.1.3 Day ( t )](https://tc39.es/ecma262/#sec-day)
///
/// The abstract operation Day takes argument t (a finite time value) and
/// returns an integral Number. It returns the day number of the day in which t falls.
/// It performs the following steps when called:
fn day(t: f64) -> f64 {
    // 1. Return 𝔽(floor(ℝ(t / msPerDay))).
    (t / MS_PER_DAY).floor()
}

/// ### [21.4.1.4 TimeWithinDay ( t )](https://tc39.es/ecma262/#sec-timewithinday)
///
/// The abstract operation TimeWithinDay takes argument t (a finite time value)
/// and returns an integral Number in the interval from +0𝔽 (inclusive) to
/// msPerDay (exclusive). It returns the number of milliseconds since the start
/// of the day in which t falls. It performs the following steps when called:
fn time_within_day(t: f64) -> f64 {
    // 1. Return 𝔽(ℝ(t) modulo ℝ(msPerDay)).
    t.rem_euclid(MS_PER_DAY)
}

/// ### [21.4.1.5 DaysInYear ( y )](https://tc39.es/ecma262/#sec-daysinyear)
///
/// The abstract operation DaysInYear takes argument y (an integral Number) and
/// returns 365𝔽 or 366𝔽. It returns the number of days in year y. Leap years
/// have 366 days; all other years have 365.
/// It performs the following steps when called:
fn days_in_year(y: i32) -> u16 {
    // 1. Let ry be ℝ(y).
    let ry = y;
    // 2. If (ry modulo 400) = 0, return 366𝔽.
    if ry % 400 == 0 {
        return 366;
    }
    // 3. If (ry modulo 100) = 0, return 365𝔽.
    if ry % 100 == 0 {
        return 365;
    }
    // 4. If (ry modulo 4) = 0, return 366𝔽.
    if ry % 4 == 0 {
        return 366;
    }
    // 5. Return 365𝔽.
    return 365;
}

/// ### [21.4.1.6 DayFromYear ( y )](https://tc39.es/ecma262/#sec-dayfromyear)
///
/// The abstract operation DayFromYear takes argument y (an integral Number)
/// and returns an integral Number. It returns the day number of the first day
/// of year y. It performs the following steps when called:
fn day_from_year(y: f64) -> f64 {
    // 1. Let ry be ℝ(y).
    let ry = y;
    // 2. NOTE: In the following steps,
    //    numYears1, numYears4, numYears100, and numYears400 represent
    //    the number of years divisible by 1, 4, 100, and 400, respectively,
    //    that occur between the epoch and the start of year y.
    //    The number is negative if y is before the epoch.

    // 3. Let numYears1 be (ry - 1970).
    let num_years_1 = ry - 1970.0;

    // 4. Let numYears4 be floor((ry - 1969) / 4).
    let num_years_4 = ((ry - 1969.0) / 4.0).floor();

    // 5. Let numYears100 be floor((ry - 1901) / 100).
    let num_years_100 = ((ry - 1901.0) / 100.0).floor();

    // 6. Let numYears400 be floor((ry - 1601) / 400).
    let num_years_400 = ((ry - 1601.0) / 400.0).floor();

    // 7. Return 𝔽(365 × numYears1 + numYears4 - numYears100 + numYears400).
    365.0 * num_years_1 + num_years_4 - num_years_100 + num_years_400
}

/// ### [21.4.1.7 TimeFromYear ( y )](https://tc39.es/ecma262/#sec-timefromyear)
///
/// The abstract operation TimeFromYear takes argument y (an integral Number)
/// and returns a time value. It returns the time value of the start of year y.
/// It performs the following steps when called:
fn time_from_year(y: f64) -> f64 {
    // 1. Return msPerDay × DayFromYear(y).
    MS_PER_DAY * day_from_year(y)
}

/// ### [21.4.1.8 YearFromTime ( t )](https://tc39.es/ecma262/#sec-yearfromtime)
///
/// The abstract operation YearFromTime takes argument t (a finite time value)
/// and returns an integral Number. It returns the year in which t falls. It
/// performs the following steps when called:
fn year_from_time(t: f64) -> i32 {
    // 1. Return the largest integral Number y (closest to +∞) such that TimeFromYear(y) ≤ t.
    let year = t / 31557600000.0;
    year.floor() as i32
}

/// ### [21.4.1.9 DayWithinYear ( t )](https://tc39.es/ecma262/#sec-daywithinyear)
///
/// The abstract operation DayWithinYear takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 365𝔽.
/// It performs the following steps when called:
fn day_within_year(t: f64) -> u16 {
    // 1. Return Day(t) - DayFromYear(YearFromTime(t)).
    (day(t) - day_from_year(year_from_time(t).into())) as u16
}

/// ### [21.4.1.10 InLeapYear ( t )](https://tc39.es/ecma262/#sec-inleapyear)
///
/// The abstract operation InLeapYear takes argument t (a finite time value)
/// and returns +0𝔽 or 1𝔽. It returns 1𝔽 if t is within a leap year and +0𝔽
/// otherwise. It performs the following steps when called:
fn in_leap_year(t: f64) -> u16 {
    // 1. If DaysInYear(YearFromTime(t)) is 366𝔽, return 1𝔽; else return +0𝔽.
    if days_in_year(year_from_time(t)) == 366 {
        1
    } else {
        0
    }
}

/// ### [21.4.1.11 MonthFromTime ( t )](https://tc39.es/ecma262/#sec-monthfromtime)
///
/// The abstract operation MonthFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 11𝔽.
/// It returns a Number identifying the month in which t falls. A month value
/// of +0𝔽 specifies January; 1𝔽 specifies February; 2𝔽 specifies March; 3𝔽
/// specifies April; 4𝔽 specifies May; 5𝔽 specifies June; 6𝔽 specifies July;
/// 7𝔽 specifies August; 8𝔽 specifies September; 9𝔽 specifies October; 10𝔽
/// specifies November; and 11𝔽 specifies December. Note that MonthFromTime
/// (+0𝔽) = +0𝔽, corresponding to Thursday, 1 January 1970. It performs the
/// following steps when called:
fn month_from_time(t: f64) -> u8 {
    // 1. Let inLeapYear be InLeapYear(t).
    let in_leap_year = in_leap_year(t);

    // 2. Let dayWithinYear be DayWithinYear(t).
    let day_within_year = day_within_year(t);

    match day_within_year {
        // 3. If dayWithinYear < 31𝔽, return +0𝔽.
        t if t < 31 => 0,
        // 4. If dayWithinYear < 59𝔽 + inLeapYear, return 1𝔽.
        t if t < 59 + in_leap_year => 1,
        // 5. If dayWithinYear < 90𝔽 + inLeapYear, return 2𝔽.
        t if t < 90 + in_leap_year => 2,
        // 6. If dayWithinYear < 120𝔽 + inLeapYear, return 3𝔽.
        t if t < 120 + in_leap_year => 3,
        // 7. If dayWithinYear < 151𝔽 + inLeapYear, return 4𝔽.
        t if t < 151 + in_leap_year => 4,
        // 8. If dayWithinYear < 181𝔽 + inLeapYear, return 5𝔽.
        t if t < 181 + in_leap_year => 5,
        // 9. If dayWithinYear < 212𝔽 + inLeapYear, return 6𝔽.
        t if t < 212 + in_leap_year => 6,
        // 10. If dayWithinYear < 243𝔽 + inLeapYear, return 7𝔽.
        t if t < 243 + in_leap_year => 7,
        // 11. If dayWithinYear < 273𝔽 + inLeapYear, return 8𝔽.
        t if t < 273 + in_leap_year => 8,
        // 12. If dayWithinYear < 304𝔽 + inLeapYear, return 9𝔽.
        t if t < 304 + in_leap_year => 9,
        // 13. If dayWithinYear < 334𝔽 + inLeapYear, return 10𝔽.
        t if t < 334 + in_leap_year => 10,
        // 14. Assert: dayWithinYear < 365𝔽 + inLeapYear.
        // 15. Return 11𝔽.
        _ => 11,
    }
}

/// ### [21.4.1.12 DateFromTime ( t )](https://tc39.es/ecma262/#sec-datefromtime)
///
/// The abstract operation DateFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from 1𝔽 to 31𝔽.
/// It returns the day of the month in which t falls.
/// It performs the following steps when called:
fn date_from_time(t: f64) -> u8 {
    // 1. Let inLeapYear be InLeapYear(t).
    let in_leap_year = in_leap_year(t);

    // 2. Let dayWithinYear be DayWithinYear(t).
    let day_within_year = day_within_year(t);

    // 3. Let month be MonthFromTime(t).
    let month = month_from_time(t);

    let date = match month {
        // 4. If month is +0𝔽, return dayWithinYear + 1𝔽.
        0 => day_within_year + 1,
        // 5. If month is 1𝔽, return dayWithinYear - 30𝔽.
        1 => day_within_year - 30,
        // 6. If month is 2𝔽, return dayWithinYear - 58𝔽 - inLeapYear.
        2 => day_within_year - 58 - in_leap_year,
        // 7. If month is 3𝔽, return dayWithinYear - 89𝔽 - inLeapYear.
        3 => day_within_year - 89 - in_leap_year,
        // 8. If month is 4𝔽, return dayWithinYear - 119𝔽 - inLeapYear.
        4 => day_within_year - 119 - in_leap_year,
        // 9. If month is 5𝔽, return dayWithinYear - 150𝔽 - inLeapYear.
        5 => day_within_year - 150 - in_leap_year,
        // 10. If month is 6𝔽, return dayWithinYear - 180𝔽 - inLeapYear.
        6 => day_within_year - 180 - in_leap_year,
        // 11. If month is 7𝔽, return dayWithinYear - 211𝔽 - inLeapYear.
        7 => day_within_year - 211 - in_leap_year,
        // 12. If month is 8𝔽, return dayWithinYear - 242𝔽 - inLeapYear.
        8 => day_within_year - 242 - in_leap_year,
        // 13. If month is 9𝔽, return dayWithinYear - 272𝔽 - inLeapYear.
        9 => day_within_year - 272 - in_leap_year,
        // 14. If month is 10𝔽, return dayWithinYear - 303𝔽 - inLeapYear.
        10 => day_within_year - 303 - in_leap_year,
        // 15. Assert: month is 11𝔽.
        // 16. Return dayWithinYear - 333𝔽 - inLeapYear.
        _ => day_within_year - 333 - in_leap_year,
    };
    date as u8
}

/// ### [21.4.1.13 WeekDay ( t )](https://tc39.es/ecma262/#sec-weekday)
///
/// The abstract operation WeekDay takes argument t (a finite time value) and
/// returns an integral Number in the inclusive interval from +0𝔽 to 6𝔽.
/// It returns a Number identifying the day of the week in which t falls.
/// A weekday value of +0𝔽 specifies Sunday; 1𝔽 specifies Monday;
/// 2𝔽 specifies Tuesday; 3𝔽 specifies Wednesday; 4𝔽 specifies Thursday;
/// 5𝔽 specifies Friday; and 6𝔽 specifies Saturday.
/// Note that WeekDay(+0𝔽) = 4𝔽, corresponding to Thursday, 1 January 1970.
///  It performs the following steps when called:
fn week_day(t: f64) -> u8 {
    // 1. Return 𝔽(ℝ(Day(t) + 4𝔽) modulo 7).
    (day(t) + 4.0).rem_euclid(7.0) as u8
}

/// ### [21.4.1.14 HourFromTime ( t )](https://tc39.es/ecma262/#sec-hourfromtime)
///
/// The abstract operation HourFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 23𝔽.
/// It returns the hour of the day in which t falls.
/// It performs the following steps when called:
fn hour_from_time(t: f64) -> u8 {
    // 1. Return 𝔽(floor(ℝ(t / msPerHour)) modulo HoursPerDay).
    ((t / MS_PER_HOUR).floor()).rem_euclid(HOURS_PER_DAY) as u8
}

/// ### [21.4.1.15 MinFromTime ( t )](https://tc39.es/ecma262/#sec-minfromtime)
///
/// The abstract operation MinFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 59𝔽.
/// It returns the minute of the hour in which t falls.
/// It performs the following steps when called:
pub(super) fn min_from_time(t: f64) -> u8 {
    // 1. Return 𝔽(floor(ℝ(t / msPerMinute)) modulo MinutesPerHour).
    ((t / MS_PER_MINUTE).floor()).rem_euclid(MINUTES_PER_HOUR) as u8
}

/// ### [21.4.1.16 SecFromTime ( t )](https://tc39.es/ecma262/#sec-secfrotime)
///
/// The abstract operation SecFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 59𝔽.
/// It returns the second of the minute in which t falls.
/// It performs the following steps when called:
fn sec_from_time(t: f64) -> u8 {
    // 1. Return 𝔽(floor(ℝ(t / msPerSecond)) modulo SecondsPerMinute).
    ((t / MS_PER_SECOND).floor()).rem_euclid(SECONDS_PER_MINUTE) as u8
}

/// ### [21.4.1.17 msFromTime ( t )](https://tc39.es/ecma262/#sec-msfromtime)
///
/// The abstract operation msFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 999𝔽.
/// It returns the millisecond of the second in which t falls.
/// It performs the following steps when called:
fn ms_from_time(t: f64) -> u16 {
    // 1. Return 𝔽(ℝ(t) modulo ℝ(msPerSecond)).
    (t.rem_euclid(MS_PER_SECOND)) as u16
}

/// ### [21.4.1.18 GetUTCEpochNanoseconds ( year, month, day, hour, minute, second, millisecond, microsecond, nanosecond )](https://tc39.es/ecma262/#sec-getutcepochnanoseconds)
///
/// The abstract operation GetUTCEpochNanoseconds takes arguments year
/// (an integer), month (an integer in the inclusive interval from 1 to 12),
/// day (an integer in the inclusive interval from 1 to 31), hour (an integer
/// in the inclusive interval from 0 to 23), minute (an integer in the
/// inclusive interval from 0 to 59), second (an integer in the inclusive
/// interval from 0 to 59), millisecond (an integer in the inclusive interval
/// from 0 to 999), microsecond (an integer in the inclusive interval from 0 to
/// 999), and nanosecond (an integer in the inclusive interval from 0 to 999)
/// and returns a BigInt. The returned value represents a number of nanoseconds
/// since the epoch that corresponds to the given ISO 8601 calendar date and
/// wall-clock time in UTC. It performs the following steps when called:
fn get_utc_epoch_nanoseconds(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u16,
    microsecond: u16,
    nanosecond: u16,
) -> i64 {
    // 1. Let date be MakeDay(𝔽(year), 𝔽(month - 1), 𝔽(day)).
    // 2. Let time be MakeTime(𝔽(hour), 𝔽(minute), 𝔽(second), 𝔽(millisecond)).
    // 3. Let ms be MakeDate(date, time).
    // 4. Assert: ms is an integral Number.
    // 5. Return ℤ(ℝ(ms) × 10**6 + microsecond × 10**3 + nanosecond).
    todo!()
}

/// ### [21.4.1.25 LocalTime ( t )](https://tc39.es/ecma262/#sec-localtime)
///
/// The abstract operation LocalTime takes argument t (a finite time value) and
/// returns an integral Number. It converts t from UTC to local time. The local
/// political rules for standard time and daylight saving time in effect at t
/// should be used to determine the result in the way specified in this
/// section. It performs the following steps when called:
///
/// > ### Note 1
/// >
/// > If political rules for the local time t are not available within the
/// > implementation, the result is t because SystemTimeZoneIdentifier returns
/// > "UTC" and GetNamedTimeZoneOffsetNanoseconds returns 0.
/// >
/// > ### Note 2
/// >
/// > It is required for time zone aware implementations (and recommended for
/// > all others) to use the time zone information of the IANA Time Zone
/// > Database https://www.iana.org/time-zones/.
/// >
/// > ### Note 3
/// >
/// > Two different input time values tUTC are converted to the same local time
/// > tlocal at a negative time zone transition when there are repeated times
/// > (e.g. the daylight saving time ends or the time zone adjustment is
/// > decreased.).
/// >
/// > LocalTime(UTC(tlocal)) is not necessarily always equal to tlocal.
/// > Correspondingly, UTC(LocalTime(tUTC)) is not necessarily always equal to tUTC.
fn local_time<'a>(agent: &mut Agent, t: SystemTime) -> f64 {
    // 1. Let systemTimeZoneIdentifier be SystemTimeZoneIdentifier().
    // 2. If IsTimeZoneOffsetString(systemTimeZoneIdentifier) is true, then
    //   a. Let offsetNs be ParseTimeZoneOffsetString(systemTimeZoneIdentifier).
    // 3. Else,
    //   a. Let offsetNs be GetNamedTimeZoneOffsetNanoseconds(systemTimeZoneIdentifier, ℤ(ℝ(t) × 10**6)).
    // 4. Let offsetMs be truncate(offsetNs / 10**6).
    // 5. Return t + 𝔽(offsetMs).
    todo!()
}
