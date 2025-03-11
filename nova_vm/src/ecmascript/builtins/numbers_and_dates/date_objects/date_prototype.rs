// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            operations_on_objects::invoke,
            type_conversion::{
                PreferredType, ordinary_to_primitive, to_integer_or_infinity_f64, to_object,
            },
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic,
            date::{
                Date,
                data::{DateValue, time_clip},
            },
            text_processing::string_objects::string_prototype::to_zero_padded_decimal_string,
        },
        execution::{Agent, JsResult, RealmIdentifier, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, Number, Object, PropertyKey, String, Value},
    },
    engine::rootable::Scopable,
    heap::{IntrinsicFunctionIndexes, WellKnownSymbolIndexes},
};
use crate::{
    ecmascript::abstract_operations::type_conversion::to_number,
    engine::context::{Bindable, GcScope, NoGcScope},
};

pub(crate) struct DatePrototype;

struct DatePrototypeGetDate;
impl Builtin for DatePrototypeGetDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getDate;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_date::<false>);
}
struct DatePrototypeGetDay;
impl Builtin for DatePrototypeGetDay {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getDay;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_day::<false>);
}
struct DatePrototypeGetFullYear;
impl Builtin for DatePrototypeGetFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_full_year::<false>);
}
struct DatePrototypeGetHours;
impl Builtin for DatePrototypeGetHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_hours::<false>);
}
struct DatePrototypeGetMilliseconds;
impl Builtin for DatePrototypeGetMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_milliseconds::<false>);
}
struct DatePrototypeGetMinutes;
impl Builtin for DatePrototypeGetMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_minutes::<false>);
}
struct DatePrototypeGetMonth;
impl Builtin for DatePrototypeGetMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_month::<false>);
}
struct DatePrototypeGetSeconds;
impl Builtin for DatePrototypeGetSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_seconds::<false>);
}
struct DatePrototypeGetTime;
impl Builtin for DatePrototypeGetTime {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getTime;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_time::<false>);
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
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_date::<true>);
}
struct DatePrototypeGetUTCDay;
impl Builtin for DatePrototypeGetUTCDay {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCDay;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_day::<true>);
}
struct DatePrototypeGetUTCFullYear;
impl Builtin for DatePrototypeGetUTCFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_full_year::<true>);
}
struct DatePrototypeGetUTCHours;
impl Builtin for DatePrototypeGetUTCHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_hours::<true>);
}
struct DatePrototypeGetUTCMilliseconds;
impl Builtin for DatePrototypeGetUTCMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_milliseconds::<true>);
}
struct DatePrototypeGetUTCMinutes;
impl Builtin for DatePrototypeGetUTCMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_minutes::<true>);
}
struct DatePrototypeGetUTCMonth;
impl Builtin for DatePrototypeGetUTCMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_month::<true>);
}
struct DatePrototypeGetUTCSeconds;
impl Builtin for DatePrototypeGetUTCSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUTCSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_seconds::<true>);
}
struct DatePrototypeSetDate;
impl Builtin for DatePrototypeSetDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setDate;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_date::<false>);
}
struct DatePrototypeSetFullYear;
impl Builtin for DatePrototypeSetFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setFullYear;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_full_year::<false>);
}
struct DatePrototypeSetHours;
impl Builtin for DatePrototypeSetHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setHours;
    const LENGTH: u8 = 4;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_hours::<false>);
}
struct DatePrototypeSetMilliseconds;
impl Builtin for DatePrototypeSetMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setMilliseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_milliseconds::<false>);
}
struct DatePrototypeSetMinutes;
impl Builtin for DatePrototypeSetMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setMinutes;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_minutes::<false>);
}
struct DatePrototypeSetMonth;
impl Builtin for DatePrototypeSetMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setMonth;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_month::<false>);
}
struct DatePrototypeSetSeconds;
impl Builtin for DatePrototypeSetSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setSeconds;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_seconds::<false>);
}
struct DatePrototypeSetTime;
impl Builtin for DatePrototypeSetTime {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setTime;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_time::<false>);
}
struct DatePrototypeSetUTCDate;
impl Builtin for DatePrototypeSetUTCDate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCDate;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_date::<true>);
}
struct DatePrototypeSetUTCFullYear;
impl Builtin for DatePrototypeSetUTCFullYear {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCFullYear;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_full_year::<true>);
}
struct DatePrototypeSetUTCHours;
impl Builtin for DatePrototypeSetUTCHours {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCHours;
    const LENGTH: u8 = 4;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_hours::<true>);
}
struct DatePrototypeSetUTCMilliseconds;
impl Builtin for DatePrototypeSetUTCMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCMilliseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_milliseconds::<true>);
}
struct DatePrototypeSetUTCMinutes;
impl Builtin for DatePrototypeSetUTCMinutes {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCMinutes;
    const LENGTH: u8 = 3;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_minutes::<true>);
}
struct DatePrototypeSetUTCMonth;
impl Builtin for DatePrototypeSetUTCMonth {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCMonth;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_month::<true>);
}
struct DatePrototypeSetUTCSeconds;
impl Builtin for DatePrototypeSetUTCSeconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUTCSeconds;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_seconds::<true>);
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
    fn get_date<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return DateFromTime(LocalTime(t)).
        Ok(Value::Integer(
            date_from_time(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.3 Date.prototype.getDay ( )](https://tc39.es/ecma262/#sec-date.prototype.getday)
    fn get_day<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return WeekDay(LocalTime(t)).
        Ok(Value::Integer(
            week_day(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.4 Date.prototype.getFullYear ( )](https://tc39.es/ecma262/#sec-date.prototype.getfullyear)
    fn get_full_year<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return YearFromTime(LocalTime(t)).
        Ok(Value::Integer(
            year_from_time(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.5 Date.prototype.getHours ( )](https://tc39.es/ecma262/#sec-date.prototype.gethours)
    fn get_hours<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return HourFromTime(LocalTime(t)).
        Ok(Value::Integer(
            hour_from_time(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.6 Date.prototype.getMilliseconds ( )](https://tc39.es/ecma262/#sec-date.prototype.getmilliseconds)
    fn get_milliseconds<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return msFromTime(LocalTime(t)).
        Ok(Value::Integer(
            ms_from_time(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.7 Date.prototype.getMinutes ( )](https://tc39.es/ecma262/#sec-date.prototype.getminutes)
    fn get_minutes<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return MinFromTime(LocalTime(t)).
        Ok(Value::Integer(
            min_from_time(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.8 Date.prototype.getMonth ( )](https://tc39.es/ecma262/#sec-date.prototype.getmonth)
    fn get_month<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return MonthFromTime(LocalTime(t)).
        Ok(Value::Integer(
            month_from_time(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.9 Date.prototype.getSeconds ( )](https://tc39.es/ecma262/#sec-date.prototype.getseconds)
    fn get_seconds<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return SecFromTime(LocalTime(t)).
        Ok(Value::Integer(
            sec_from_time(local_or_utc_time::<UTC>(agent, t)).into(),
        ))
    }

    /// ### [21.4.4.10 Date.prototype.getTime ( )](https://tc39.es/ecma262/#sec-date.prototype.gettime)
    fn get_time<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Return dateObject.[[DateValue]].
        Ok(date_object.date_value(agent).into_value())
    }

    fn get_timezone_offset<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.date_value(agent);
        // 4. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 5. Return (t - LocalTime(t)) / msPerMinute.
        let result = (t - local_time(agent, t)) / MS_PER_MINUTE;
        Ok(Value::from_f64(agent, result, gc.into_nogc()))
    }

    /// ### [21.4.4.20 Date.prototype.setDate ( date )](https://tc39.es/ecma262/#sec-date.prototype.setdate)
    fn set_date<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let date = arguments.get(0).bind(gc.nogc());
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.get(agent).date_value(agent);
        // 4. Let dt be ? ToNumber(date).
        let dt = to_number(agent, date.unbind(), gc.reborrow())?.to_real(agent);
        // 5. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 6. Set t to LocalTime(t).
        let t = local_or_utc_time::<UTC>(agent, t);
        // 7. Let newDate be MakeDate(MakeDay(YearFromTime(t), MonthFromTime(t), dt), TimeWithinDay(t)).
        let new_date = make_date(
            make_day(year_from_time(t) as f64, month_from_time(t) as f64, dt),
            time_within_day(t),
        );
        // 8. Let u be TimeClip(UTC(newDate)).
        let u = time_clip(utc(agent, new_date));
        // 9. Set dateObject.[[DateValue]] to u.
        date_object.get(agent).set_date_value(agent, u);
        // 10. Return u.
        Ok(u.into_value())
    }

    /// ### [21.4.4.21 Date.prototype.setFullYear ( year \[ , month \[ , date \] \] )](https://tc39.es/ecma262/#sec-date.prototype.setfullyear)
    ///
    /// The "length" property of this method is 3𝔽.
    ///
    /// > #### Note
    /// >
    /// > If month is not present, this method behaves as if month was present
    /// > with the value getMonth(). If date is not present, it behaves as if
    /// > date was present with the value getDate().
    fn set_full_year<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let year = arguments.get(0).bind(gc.nogc());
        let month = arguments.get(1).scope(agent, gc.nogc());
        let date = arguments.get(2).scope(agent, gc.nogc());
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.get(agent).date_value(agent);
        // 4. Let y be ? ToNumber(year).
        let y = to_number(agent, year.unbind(), gc.reborrow())?.to_real(agent);
        // 5. If t is NaN, set t to +0𝔽; otherwise, set t to LocalTime(t).
        let t = if let Some(t) = t.get_f64() {
            local_or_utc_time::<UTC>(agent, t)
        } else {
            0.0
        };
        // 6. If month is not present, let m be MonthFromTime(t); otherwise, let m be ? ToNumber(month).
        let m = if arguments.len() < 2 {
            month_from_time(t) as f64
        } else {
            month
                .get(agent)
                .to_number(agent, gc.reborrow())?
                .into_f64(agent)
        };
        // 7. If date is not present, let dt be DateFromTime(t); otherwise, let dt be ? ToNumber(date).
        let dt = if arguments.len() < 3 {
            date_from_time(t) as f64
        } else {
            date.get(agent)
                .to_number(agent, gc.reborrow())?
                .into_f64(agent)
        };
        // 8. Let newDate be MakeDate(MakeDay(y, m, dt), TimeWithinDay(t)).
        let new_date = make_date(make_day(y, m, dt), time_within_day(t));
        // 9. Let u be TimeClip(UTC(newDate)).
        let u = time_clip(utc(agent, new_date));
        // 10. Set dateObject.[[DateValue]] to u.
        date_object.get(agent).set_date_value(agent, u);
        // 11. Return u.
        Ok(u.into_value())
    }

    /// ### [21.4.4.22 Date.prototype.setHours ( hour \[ , min \[ , sec \[ , ms \] \] \] )](https://tc39.es/ecma262/#sec-date.prototype.sethours)
    ///
    /// The "length" property of this method is 4𝔽.
    ///
    /// > #### Note
    /// >
    /// > If min is not present, this method behaves as if min was present with
    /// > the value getMinutes(). If sec is not present, it behaves as if sec
    /// > was present with the value getSeconds(). If ms is not present, it
    /// > behaves as if ms was present with the value getMilliseconds().
    fn set_hours<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let hour = arguments.get(0).bind(gc.nogc());
        let min = arguments.get(1).scope(agent, gc.nogc());
        let sec = arguments.get(2).scope(agent, gc.nogc());
        let ms = arguments.get(3).scope(agent, gc.nogc());
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.get(agent).date_value(agent);
        // 4. Let h be ? ToNumber(hour).
        let h = to_number(agent, hour.unbind(), gc.reborrow())?.to_real(agent);
        // 5. If min is present, let m be ? ToNumber(min).
        let m = if arguments.len() > 1 {
            Some(to_number(agent, min.get(agent), gc.reborrow())?.to_real(agent))
        } else {
            None
        };
        // 6. If sec is present, let s be ? ToNumber(sec).
        let s = if arguments.len() > 2 {
            Some(to_number(agent, sec.get(agent), gc.reborrow())?.to_real(agent))
        } else {
            None
        };
        // 7. If ms is present, let milli be ? ToNumber(ms).
        let milli = if arguments.len() > 3 {
            Some(to_number(agent, ms.get(agent), gc.reborrow())?.to_real(agent))
        } else {
            None
        };
        // 8. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 9. Set t to LocalTime(t).
        let t = local_or_utc_time::<UTC>(agent, t);
        // 10. If min is not present, let m be MinFromTime(t).
        let m = m.unwrap_or_else(|| min_from_time(t) as f64);
        // 11. If sec is not present, let s be SecFromTime(t).
        let s = s.unwrap_or_else(|| sec_from_time(t) as f64);
        // 12. If ms is not present, let milli be msFromTime(t).
        let milli = milli.unwrap_or_else(|| ms_from_time(t) as f64);
        // 13. Let date be MakeDate(Day(t), MakeTime(h, m, s, milli)).
        let date = make_date(day(t), make_time(h, m, s, milli));
        // 14. Let u be TimeClip(UTC(date)).
        let u = time_clip(utc(agent, date));
        // 15. Set dateObject.[[DateValue]] to u.
        date_object.get(agent).set_date_value(agent, u);
        // 16. Return u.
        Ok(u.into_value())
    }

    /// ### [21.4.4.23 Date.prototype.setMilliseconds ( ms )](https://tc39.es/ecma262/#sec-date.prototype.setmilliseconds)
    fn set_milliseconds<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.get(agent).date_value(agent);
        // 4. Set ms to ? ToNumber(ms).
        let ms = to_number(agent, arguments.get(0), gc.reborrow())?.to_real(agent);
        // 5. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 6. Set t to LocalTime(t).
        let t = local_or_utc_time::<UTC>(agent, t);
        // 7. Let time be MakeTime(HourFromTime(t), MinFromTime(t), SecFromTime(t), ms).
        let time = make_time(
            hour_from_time(t) as f64,
            min_from_time(t) as f64,
            sec_from_time(t) as f64,
            ms,
        );
        // 8. Let u be TimeClip(UTC(MakeDate(Day(t), time))).
        let u = time_clip(utc(agent, make_date(day(t), time)));
        // 9. Set dateObject.[[DateValue]] to u.
        date_object.get(agent).set_date_value(agent, u);
        // 10. Return u.
        Ok(u.into_value())
    }

    /// ### [21.4.4.24 Date.prototype.setMinutes ( min \[ , sec \[ , ms \] \] )](https://tc39.es/ecma262/#sec-date.prototype.setminutes)
    ///
    /// The "length" property of this method is 3𝔽.
    ///
    /// > #### Note
    /// >
    /// > If sec is not present, this method behaves as if sec was present with
    /// > the value getSeconds(). If ms is not present, this behaves as if ms
    /// > was present with the value getMilliseconds().
    fn set_minutes<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let min = arguments.get(0).bind(gc.nogc());
        let sec = arguments.get(1).scope(agent, gc.nogc());
        let ms = arguments.get(2).scope(agent, gc.nogc());
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.get(agent).date_value(agent);
        // 4. Let m be ? ToNumber(min).
        let m = to_number(agent, min.unbind(), gc.reborrow())?.to_real(agent);
        // 5. If sec is present, let s be ? ToNumber(sec).
        let s = if arguments.len() > 1 {
            Some(to_number(agent, sec.get(agent), gc.reborrow())?.to_real(agent))
        } else {
            None
        };
        // 6. If ms is present, let milli be ? ToNumber(ms).
        let milli = if arguments.len() > 2 {
            Some(to_number(agent, ms.get(agent), gc.reborrow())?.to_real(agent))
        } else {
            None
        };
        // 7. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 8. Set t to LocalTime(t).
        let t = local_or_utc_time::<UTC>(agent, t);
        // 9. If sec is not present, let s be SecFromTime(t).
        let s = s.unwrap_or_else(|| sec_from_time(t) as f64);
        // 10. If ms is not present, let milli be msFromTime(t).
        let milli = milli.unwrap_or_else(|| ms_from_time(t) as f64);
        // 11. Let date be MakeDate(Day(t), MakeTime(HourFromTime(t), m, s, milli)).
        let date = make_date(day(t), make_time(hour_from_time(t) as f64, m, s, milli));
        // 12. Let u be TimeClip(UTC(date)).
        let u = time_clip(utc(agent, date));
        // 13. Set dateObject.[[DateValue]] to u.
        date_object.get(agent).set_date_value(agent, u);
        // 14. Return u.
        Ok(u.into_value())
    }

    /// ### [21.4.4.25 Date.prototype.setMonth ( month \[ , date \] )](https://tc39.es/ecma262/#sec-date.prototype.setmonth)
    ///
    /// The "length" property of this method is 2𝔽.
    ///
    /// > #### Note
    /// >
    /// > If date is not present, this method behaves as if date was present
    /// > with the value getDate().
    fn set_month<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let month = arguments.get(0).bind(gc.nogc());
        let date = arguments.get(1).scope(agent, gc.nogc());
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.get(agent).date_value(agent);
        // 4. Let m be ? ToNumber(month).
        let m = to_number(agent, month.unbind(), gc.reborrow())?.to_real(agent);
        // 5. If date is present, let dt be ? ToNumber(date).
        let dt = if arguments.len() > 1 {
            Some(to_number(agent, date.get(agent), gc.reborrow())?.to_real(agent))
        } else {
            None
        };
        // 6. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 7. Set t to LocalTime(t).
        let t = local_or_utc_time::<UTC>(agent, t);
        // 8. If date is not present, let dt be DateFromTime(t).
        let dt = dt.unwrap_or_else(|| date_from_time(t) as f64);
        // 9. Let newDate be MakeDate(MakeDay(YearFromTime(t), m, dt), TimeWithinDay(t)).
        let new_date = make_date(
            make_day(year_from_time(t) as f64, m, dt),
            time_within_day(t),
        );
        // 10. Let u be TimeClip(UTC(newDate)).
        let u = time_clip(utc(agent, new_date));
        // 11. Set dateObject.[[DateValue]] to u.
        date_object.get(agent).set_date_value(agent, u);
        // 12. Return u.
        Ok(u.into_value())
    }

    /// ### [21.4.4.26 Date.prototype.setSeconds ( sec \[ , ms \] )](https://tc39.es/ecma262/#sec-date.prototype.setseconds)
    ///
    /// The "length" property of this method is 2𝔽.
    ///
    /// > #### Note
    /// >
    /// > If ms is not present, this method behaves as if ms was present with
    /// > the value getMilliseconds().
    fn set_seconds<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let sec = arguments.get(0).bind(gc.nogc());
        let ms = arguments.get(1).scope(agent, gc.nogc());
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be dateObject.[[DateValue]].
        let t = date_object.get(agent).date_value(agent);
        // 4. Let s be ? ToNumber(sec).
        let s = to_number(agent, sec.unbind(), gc.reborrow())?.to_real(agent);
        // 5. If ms is present, let milli be ? ToNumber(ms).
        let milli = if arguments.len() > 1 {
            Some(to_number(agent, ms.get(agent), gc.reborrow())?.to_real(agent))
        } else {
            None
        };
        // 6. If t is NaN, return NaN.
        let Some(t) = t.get_f64() else {
            return Ok(Value::nan());
        };
        // 7. Set t to LocalTime(t).
        let t = local_or_utc_time::<UTC>(agent, t);
        // 8. If ms is not present, let milli be msFromTime(t).
        let milli = milli.unwrap_or_else(|| ms_from_time(t) as f64);
        // 9. Let date be MakeDate(Day(t), MakeTime(HourFromTime(t), MinFromTime(t), s, milli)).
        let date = make_date(
            day(t),
            make_time(hour_from_time(t) as f64, min_from_time(t) as f64, s, milli),
        );
        // 10. Let u be TimeClip(UTC(date)).
        let u = time_clip(utc(agent, date));
        // 11. Set dateObject.[[DateValue]] to u.
        date_object.get(agent).set_date_value(agent, u);
        // 12. Return u.
        Ok(u.into_value())
    }

    /// ### [21.4.4.27 Date.prototype.setTime ( time )](https://tc39.es/ecma262/#sec-date.prototype.settime)
    fn set_time<'gc, const UTC: bool>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        let date_object = date_object.scope(agent, gc.nogc());
        // 3. Let t be ? ToNumber(time).
        let t = to_number(agent, arguments.get(0), gc.reborrow())?.to_real(agent);
        // 4. Let v be TimeClip(t).
        let v = time_clip(t);
        // 5. Set dateObject.[[DateValue]] to v.
        date_object.get(agent).set_date_value(agent, v);
        // 6. Return v.
        Ok(v.into_value())
    }

    /// ### [21.4.4.35 Date.prototype.toDateString ( )](https://tc39.es/ecma262/#sec-date.prototype.todatestring)
    fn to_date_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let tv be dateObject.[[DateValue]].
        let tv = date_object.date_value(agent);
        // 4. If tv is NaN, return "Invalid Date".
        let Some(tv) = tv.get_f64() else {
            return Ok(Value::from_static_str(
                agent,
                "Invalid Date",
                gc.into_nogc(),
            ));
        };
        // 5. Let t be LocalTime(tv).
        let t = local_time(agent, tv);
        // 6. Return DateString(t).
        Ok(Value::from_string(agent, date_string(t), gc.into_nogc()))
    }

    /// ### [21.4.4.36 Date.prototype.toISOString ( )](https://tc39.es/ecma262/#sec-date.prototype.toisostring)
    fn to_iso_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let tv be dateObject.[[DateValue]].
        let tv = date_object.date_value(agent);
        // 4. If tv is NaN, throw a RangeError exception.
        let Some(tv) = tv.get_f64() else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Invalid Date",
                gc.into_nogc(),
            ));
        };
        // 5. Assert: tv is an integral Number.
        assert!(tv.fract() == 0.0);
        // 6. If tv corresponds with a year that cannot be represented in
        // the Date Time String Format, throw a RangeError exception.
        // 7. Return a String representation of tv in the Date Time String Format
        // on the UTC time scale, including all format elements and the UTC offset representation "Z".
        // (The format is "YYYY-MM-DDTHH:mm:ss.sssZ")
        let year = year_from_time(tv);
        let month = month_from_time(tv) + 1;
        let day = date_from_time(tv);
        let hour = hour_from_time(tv);
        let minute = min_from_time(tv);
        let second = sec_from_time(tv);
        let ms = ms_from_time(tv);
        let date_string = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            year, month, day, hour, minute, second, ms
        );
        Ok(Value::from_string(agent, date_string, gc.into_nogc()))
    }

    /// ### [21.4.4.37 Date.prototype.toJSON ( key )](https://tc39.es/ecma262/#sec-date.prototype.tojson)
    ///
    /// This method provides a String representation of a Date for use
    /// by JSON.stringify (25.5.2).
    ///
    /// > #### Note 1
    /// >
    /// > The argument is ignored.
    /// >
    /// > #### Note 2
    /// >
    /// > This method is intentionally generic; it does not require that its
    /// > this value be a Date. Therefore, it can be transferred to other kinds
    /// > of objects for use as a method. However, it does require that any
    /// > such object have a toISOString method.
    fn to_json<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be ? ToObject(this value).
        let o = to_object(agent, this_value, gc.nogc())?.scope(agent, gc.nogc());
        // 2. Let tv be ? ToPrimitive(O, number).
        let tv = ordinary_to_primitive(agent, o.get(agent), PreferredType::Number, gc.reborrow())?;
        // 3. If tv is a Number and tv is not finite, return null.
        if let Ok(tv) = Number::try_from(tv) {
            if !tv.is_finite(agent) {
                return Ok(Value::Null);
            }
        }
        // 4. Return ? Invoke(O, "toISOString").
        let k =
            PropertyKey::from_static_str(agent, "toISOString", gc.nogc()).scope(agent, gc.nogc());
        invoke(agent, o.get(agent).into_value(), k.get(agent), None, gc)
    }

    /// ### [21.4.4.38 Date.prototype.toLocaleDateString ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-date.prototype.tolocaledatestring)
    ///
    /// An ECMAScript implementation that includes the ECMA-402
    /// Internationalization API must implement this method as specified in the
    /// ECMA-402 specification. If an ECMAScript implementation does not
    /// include the ECMA-402 API the following specification of this method is used:
    ///
    /// This method returns a String value. The contents of the String are
    /// implementation-defined, but are intended to represent the “date”
    /// portion of the Date in the current time zone in a convenient,
    /// human-readable form that corresponds to the conventions of the host
    /// environment's current locale.
    ///
    /// The meaning of the optional parameters to this method are defined in
    /// the ECMA-402 specification; implementations that do not include
    /// ECMA-402 support must not use those parameter positions for anything else.
    fn to_locale_date_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    /// ### [21.4.4.39 Date.prototype.toLocaleString ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-date.prototype.tolocalestring)
    ///
    /// An ECMAScript implementation that includes the ECMA-402
    /// Internationalization API must implement this method as specified in the
    /// ECMA-402 specification. If an ECMAScript implementation does not
    /// include the ECMA-402 API the following specification of this method is used:
    ///
    /// This method returns a String value. The contents of the String are
    /// implementation-defined, but are intended to represent the Date in the
    /// current time zone in a convenient, human-readable form that corresponds
    /// to the conventions of the host environment's current locale.
    ///
    /// The meaning of the optional parameters to this method are defined in
    /// the ECMA-402 specification; implementations that do not include
    /// ECMA-402 support must not use those parameter positions for anything else.
    fn to_locale_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    /// ### [21.4.4.40 Date.prototype.toLocaleTimeString ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-date.prototype.tolocaletimestring)
    ///
    /// An ECMAScript implementation that includes the ECMA-402
    /// Internationalization API must implement this method as specified in the
    /// ECMA-402 specification. If an ECMAScript implementation does not
    /// include the ECMA-402 API the following specification of this method is used:
    ///
    /// This method returns a String value. The contents of the String are
    /// implementation-defined, but are intended to represent the “time”
    /// portion of the Date in the current time zone in a convenient,
    /// human-readable form that corresponds to the conventions of the host
    /// environment's current locale.
    ///
    /// The meaning of the optional parameters to this method are defined in
    /// the ECMA-402 specification; implementations that do not include
    /// ECMA-402 support must not use those parameter positions for anything else.
    fn to_locale_time_string<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    /// ### [21.4.4.41 Date.prototype.toString ( )](https://tc39.es/ecma262/#sec-date.prototype.tostring)
    ///
    /// > #### Note 1
    /// >
    /// > For any Date d such that d.[[DateValue]] is evenly divisible by 1000,
    /// > the result of Date.parse(d.toString()) = d.valueOf(). See 21.4.3.2.
    /// >
    /// > #### Note 2
    /// >
    /// > This method is not generic; it throws a TypeError exception if its
    /// > this value is not a Date. Therefore, it cannot be transferred to
    /// > other kinds of objects for use as a method.
    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let tv be dateObject.[[DateValue]].
        let tv = date_object.date_value(agent);
        // 4. Return ToDateString(tv).
        let s = to_date_string(agent, tv);
        Ok(Value::from_string(agent, s, gc.into_nogc()))
    }

    /// ### [21.4.4.42 Date.prototype.toTimeString ( )](https://tc39.es/ecma262/#sec-date.prototype.totimestring)
    fn to_time_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let tv be dateObject.[[DateValue]].
        let tv = date_object.date_value(agent);
        // 4. If tv is NaN, return "Invalid Date".
        let Some(tv) = tv.get_f64() else {
            return Ok(Value::from_static_str(
                agent,
                "Invalid Date",
                gc.into_nogc(),
            ));
        };
        // 5. Let t be LocalTime(tv).
        let t = local_time(agent, tv);
        // 6. Return the string-concatenation of TimeString(t) and TimeZoneString(tv).
        Ok(Value::from_string(
            agent,
            format!("{}{}", time_string(t), time_zone_string(agent, tv)),
            gc.into_nogc(),
        ))
    }

    /// ### [21.4.4.43 Date.prototype.toUTCString ( )](https://tc39.es/ecma262/#sec-date.prototype.toutcstring)
    ///
    /// This method returns a String value representing the instant in time
    /// corresponding to the this value. The format of the String is based upon
    /// "HTTP-date" from RFC 7231, generalized to support the full range of
    /// times supported by ECMAScript Dates.
    fn to_utc_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]])
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Let tv be dateObject.[[DateValue]].
        let tv = date_object.date_value(agent);
        // 4. If tv is NaN, return "Invalid Date".
        let Some(tv) = tv.get_f64() else {
            return Ok(Value::from_static_str(
                agent,
                "Invalid Date",
                gc.into_nogc(),
            ));
        };
        // 5. Let weekday be the Name of the entry in Table 65 with the Number WeekDay(tv).
        let weekday = match week_day(tv) {
            0 => "Sun",
            1 => "Mon",
            2 => "Tue",
            3 => "Wed",
            4 => "Thu",
            5 => "Fri",
            6 => "Sat",
            _ => unreachable!(),
        };
        // 6. Let month be the Name of the entry in Table 66 with the Number MonthFromTime(tv).
        let month = match month_from_time(tv) {
            0 => "Jan",
            1 => "Feb",
            2 => "Mar",
            3 => "Apr",
            4 => "May",
            5 => "Jun",
            6 => "Jul",
            7 => "Aug",
            8 => "Sep",
            9 => "Oct",
            10 => "Nov",
            11 => "Dec",
            _ => unreachable!(),
        };
        // 7. Let day be ToZeroPaddedDecimalString(ℝ(DateFromTime(tv)), 2).
        let day = to_zero_padded_decimal_string(date_from_time(tv), 2);
        // 8. Let yv be YearFromTime(tv).
        let yv = year_from_time(tv);
        // 9. If yv is +0𝔽 or yv > +0𝔽, let yearSign be the empty String; otherwise, let yearSign be "-".
        let year_sign = if yv >= 0 { "" } else { "-" };
        // 10. Let paddedYear be ToZeroPaddedDecimalString(abs(ℝ(yv)), 4).
        let padded_year = to_zero_padded_decimal_string(yv.abs(), 4);
        // 11. Return the string-concatenation of weekday, ",", the code unit 0x0020 (SPACE),
        // day, the code unit 0x0020 (SPACE), month, the code unit 0x0020 (SPACE), yearSign,
        // paddedYear, the code unit 0x0020 (SPACE), and TimeString(tv).
        Ok(Value::from_string(
            agent,
            format!(
                "{weekday}, {day} {month} {year_sign}{padded_year} {}",
                time_string(tv)
            ),
            gc.into_nogc(),
        ))
    }

    /// ### [21.4.4.44 Date.prototype.valueOf ( )](https://tc39.es/ecma262/#sec-date.prototype.valueof)
    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let dateObject be the this value.
        // 2. Perform ? RequireInternalSlot(dateObject, [[DateValue]]).
        let date_object = require_internal_slot_date(agent, this_value, gc.nogc())?;
        // 3. Return dateObject.[[DateValue]].
        Ok(date_object.date_value(agent).into_value())
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
pub(super) const MS_PER_MINUTE: f64 = MS_PER_SECOND * SECONDS_PER_MINUTE;
/// msPerHour = 3600000𝔽 = msPerMinute × 𝔽(MinutesPerHour)
const MS_PER_HOUR: f64 = MS_PER_MINUTE * MINUTES_PER_HOUR;
/// msPerDay = 86400000𝔽 = msPerHour × 𝔽(HoursPerDay)
const MS_PER_DAY: f64 = MS_PER_HOUR * HOURS_PER_DAY;

/// ### [21.4.1.3 Day ( t )](https://tc39.es/ecma262/#sec-day)
///
/// The abstract operation Day takes argument t (a finite time value) and
/// returns an integral Number. It returns the day number of the day in which t falls.
fn day(t: f64) -> f64 {
    // 1. Return 𝔽(floor(ℝ(t / msPerDay))).
    (t / MS_PER_DAY).floor()
}

/// ### [21.4.1.4 TimeWithinDay ( t )](https://tc39.es/ecma262/#sec-timewithinday)
///
/// The abstract operation TimeWithinDay takes argument t (a finite time value)
/// and returns an integral Number in the interval from +0𝔽 (inclusive) to
/// msPerDay (exclusive). It returns the number of milliseconds since the start
/// of the day in which t falls.
fn time_within_day(t: f64) -> f64 {
    // 1. Return 𝔽(ℝ(t) modulo ℝ(msPerDay)).
    t.rem_euclid(MS_PER_DAY)
}

/// ### [21.4.1.5 DaysInYear ( y )](https://tc39.es/ecma262/#sec-daysinyear)
///
/// The abstract operation DaysInYear takes argument y (an integral Number) and
/// returns 365𝔽 or 366𝔽. It returns the number of days in year y. Leap years
/// have 366 days; all other years have 365.
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
    365
}

/// ### [21.4.1.6 DayFromYear ( y )](https://tc39.es/ecma262/#sec-dayfromyear)
///
/// The abstract operation DayFromYear takes argument y (an integral Number)
/// and returns an integral Number. It returns the day number of the first day
/// of year y.
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
fn time_from_year(y: f64) -> f64 {
    // 1. Return msPerDay × DayFromYear(y).
    MS_PER_DAY * day_from_year(y)
}

/// ### [21.4.1.8 YearFromTime ( t )](https://tc39.es/ecma262/#sec-yearfromtime)
///
/// Ported from Boa JS engine. Source https://github.com/boa-dev/boa/blob/13a030a0aa452e6f78e4a7e8bbc0e11b878bbd58/core/engine/src/builtins/date/utils.rs#L133
///
/// Copyright (c) 2019 Jason Williams
///
/// The abstract operation YearFromTime takes argument t (a finite time value)
/// and returns an integral Number. It returns the year in which t falls. It
/// performs the following steps when called:
fn year_from_time(t: f64) -> i32 {
    const MS_PER_AVERAGE_YEAR: f64 = 12.0 * 30.436_875 * MS_PER_DAY;

    // 1. Return the largest integral Number y (closest to +∞) such that TimeFromYear(y) ≤ t.
    let mut year = (((t + MS_PER_AVERAGE_YEAR / 2.0) / MS_PER_AVERAGE_YEAR).floor()) as i32 + 1970;
    if time_from_year(year.into()) > t {
        year -= 1;
    }
    year
}

/// ### [21.4.1.9 DayWithinYear ( t )](https://tc39.es/ecma262/#sec-daywithinyear)
///
/// The abstract operation DayWithinYear takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 365𝔽.
fn day_within_year(t: f64) -> u16 {
    // 1. Return Day(t) - DayFromYear(YearFromTime(t)).
    (day(t) - day_from_year(year_from_time(t).into())) as u16
}

/// ### [21.4.1.10 InLeapYear ( t )](https://tc39.es/ecma262/#sec-inleapyear)
///
/// The abstract operation InLeapYear takes argument t (a finite time value)
/// and returns +0𝔽 or 1𝔽. It returns 1𝔽 if t is within a leap year and +0𝔽 otherwise.
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
fn week_day(t: f64) -> u8 {
    // 1. Return 𝔽(ℝ(Day(t) + 4𝔽) modulo 7).
    (day(t) + 4.0).rem_euclid(7.0) as u8
}

/// ### [21.4.1.14 HourFromTime ( t )](https://tc39.es/ecma262/#sec-hourfromtime)
///
/// The abstract operation HourFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 23𝔽.
/// It returns the hour of the day in which t falls.
fn hour_from_time(t: f64) -> u8 {
    // 1. Return 𝔽(floor(ℝ(t / msPerHour)) modulo HoursPerDay).
    ((t / MS_PER_HOUR).floor()).rem_euclid(HOURS_PER_DAY) as u8
}

/// ### [21.4.1.15 MinFromTime ( t )](https://tc39.es/ecma262/#sec-minfromtime)
///
/// The abstract operation MinFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 59𝔽.
/// It returns the minute of the hour in which t falls.
pub(super) fn min_from_time(t: f64) -> u8 {
    // 1. Return 𝔽(floor(ℝ(t / msPerMinute)) modulo MinutesPerHour).
    ((t / MS_PER_MINUTE).floor()).rem_euclid(MINUTES_PER_HOUR) as u8
}

/// ### [21.4.1.16 SecFromTime ( t )](https://tc39.es/ecma262/#sec-secfrotime)
///
/// The abstract operation SecFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 59𝔽.
/// It returns the second of the minute in which t falls.
fn sec_from_time(t: f64) -> u8 {
    // 1. Return 𝔽(floor(ℝ(t / msPerSecond)) modulo SecondsPerMinute).
    ((t / MS_PER_SECOND).floor()).rem_euclid(SECONDS_PER_MINUTE) as u8
}

/// ### [21.4.1.17 msFromTime ( t )](https://tc39.es/ecma262/#sec-msfromtime)
///
/// The abstract operation msFromTime takes argument t (a finite time value)
/// and returns an integral Number in the inclusive interval from +0𝔽 to 999𝔽.
/// It returns the millisecond of the second in which t falls.
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
/// wall-clock time in UTC.
#[allow(clippy::too_many_arguments)]
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
    let date = make_day(year as f64, (month - 1) as f64, day as f64);
    // 2. Let time be MakeTime(𝔽(hour), 𝔽(minute), 𝔽(second), 𝔽(millisecond)).
    let time = make_time(
        hour as f64,
        minute as f64,
        second as f64,
        millisecond as f64,
    );
    // 3. Let ms be MakeDate(date, time).
    let ms = make_date(date, time);
    // 4. Assert: ms is an integral Number.
    assert!(ms.fract() == 0.0);
    // 5. Return ℤ(ℝ(ms) × 10**6 + microsecond × 10**3 + nanosecond).
    (ms * 1e6 + (microsecond as f64) * 1e3 + (nanosecond as f64)) as i64
}

/// ### [21.4.1.20 GetNamedTimeZoneEpochNanoseconds ( timeZoneIdentifier, year, month, day, hour, minute, second, millisecond, microsecond, nanosecond )](https://tc39.es/ecma262/#sec-getnamedtimezoneepochnanoseconds)
///
/// The implementation-defined abstract operation
/// GetNamedTimeZoneEpochNanoseconds takes arguments timeZoneIdentifier (a
/// String), year (an integer), month (an integer in the inclusive interval
/// from 1 to 12), day (an integer in the inclusive interval from 1 to 31),
/// hour (an integer in the inclusive interval from 0 to 23), minute (an
/// integer in the inclusive interval from 0 to 59), second (an integer in the
/// inclusive interval from 0 to 59), millisecond (an integer in the inclusive
/// interval from 0 to 999), microsecond (an integer in the inclusive interval
/// from 0 to 999), and nanosecond (an integer in the inclusive interval from 0
/// to 999) and returns a List of BigInts. Each value in the returned List
/// represents a number of nanoseconds since the epoch that corresponds to the
/// given ISO 8601 calendar date and wall-clock time in the named time zone
/// identified by timeZoneIdentifier.
///
/// When the input represents a local time occurring more than once because of
/// a negative time zone transition (e.g. when daylight saving time ends or the
/// time zone offset is decreased due to a time zone rule change), the returned
/// List will have more than one element and will be sorted by ascending
/// numerical value. When the input represents a local time skipped because of
/// a positive time zone transition (e.g. when daylight saving time begins or
/// the time zone offset is increased due to a time zone rule change), the
/// returned List will be empty. Otherwise, the returned List will have one
/// element.
///
/// The default implementation of GetNamedTimeZoneEpochNanoseconds, to be used
/// for ECMAScript implementations that do not include local political rules
/// for any time zones, performs the following steps when called:
///
/// > #### Note
/// >
/// > It is required for time zone aware implementations (and recommended for
/// > all others) to use the time zone information of the IANA Time Zone
/// > Database https://www.iana.org/time-zones/.
/// >
/// > 1:30 AM on 5 November 2017 in America/New_York is repeated twice, so
/// > GetNamedTimeZoneEpochNanoseconds("America/New_York", 2017, 11, 5, 1, 30,
/// > 0, 0, 0, 0) would return a List of length 2 in which the first element
/// > represents 05:30 UTC (corresponding with 01:30 US Eastern Daylight Time
/// > at UTC offset -04:00) and the second element represents 06:30 UTC
/// > (corresponding with 01:30 US Eastern Standard Time at UTC offset -05:00).
/// >
/// > 2:30 AM on 12 March 2017 in America/New_York does not exist, so
/// > GetNamedTimeZoneEpochNanoseconds("America/New_York", 2017, 3, 12, 2, 30,
/// > 0, 0, 0, 0) would return an empty List.
#[allow(clippy::too_many_arguments)]
fn get_named_time_zone_epoch_nanoseconds(
    time_zone_identifier: &str,
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u16,
    microsecond: u16,
    nanosecond: u16,
) -> [i64; 1] {
    // 1. Assert: timeZoneIdentifier is "UTC".
    assert_eq!(time_zone_identifier, "UTC");
    // 2. Let epochNanoseconds be GetUTCEpochNanoseconds(year, month, day, hour, minute, second, millisecond, microsecond, nanosecond).
    let epoch_nanoseconds = get_utc_epoch_nanoseconds(
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond,
        microsecond,
        nanosecond,
    );
    // 3. Return « epochNanoseconds ».
    [epoch_nanoseconds]
}

/// ### [21.4.1.21 GetNamedTimeZoneOffsetNanoseconds ( timeZoneIdentifier, epochNanoseconds )](https://tc39.es/ecma262/#sec-getnamedtimezoneoffsetnanoseconds)
///
/// The implementation-defined abstract operation
/// GetNamedTimeZoneOffsetNanoseconds takes arguments timeZoneIdentifier (a
/// String) and epochNanoseconds (a BigInt) and returns an integer.
///
/// The returned integer represents the offset from UTC of the named time zone
/// identified by timeZoneIdentifier, at the instant corresponding with
/// epochNanoseconds relative to the epoch, both in nanoseconds.
///
/// The default implementation of GetNamedTimeZoneOffsetNanoseconds, to be used
/// for ECMAScript implementations that do not include local political rules
/// for any time zones, performs the following steps when called:
///
/// > #### Note
/// >
/// > Time zone offset values may be positive or negative.
fn get_named_time_zone_offset_nanoseconds(
    time_zone_identifier: &str,
    _epoch_nanoseconds: i64,
) -> f64 {
    // 1. Assert: timeZoneIdentifier is "UTC".
    assert_eq!(time_zone_identifier, "UTC");
    // 2. Return 0.
    0.0
}

/// ### [21.4.1.24 SystemTimeZoneIdentifier ( )](https://tc39.es/ecma262/#sec-systemtimezoneidentifier)
///
/// The implementation-defined abstract operation SystemTimeZoneIdentifier
/// takes no arguments and returns a String. It returns a String representing
/// the host environment's current time zone, which is either a String
/// representing a UTC offset for which IsTimeZoneOffsetString returns true, or
/// a primary time zone identifier.
///
/// > #### Note
/// >
/// > To ensure the level of functionality that implementations commonly
/// > provide in the methods of the Date object, it is recommended that
/// > SystemTimeZoneIdentifier return an IANA time zone name corresponding to
/// > the host environment's time zone setting, if such a thing exists.
/// > GetNamedTimeZoneEpochNanoseconds and GetNamedTimeZoneOffsetNanoseconds
/// > must reflect the local political rules for standard time and daylight
/// > saving time in that time zone, if such rules exist.
/// >
/// > For example, if the host environment is a browser on a system where the
/// > user has chosen US Eastern Time as their time zone,
/// > SystemTimeZoneIdentifier returns "America/New_York".
fn system_time_zone_identifier(_agent: &Agent) -> &'static str {
    // 1. If the implementation only supports the UTC time zone, return "UTC".
    // 2. Let systemTimeZoneString be the String representing the host environment's
    // current time zone, either a primary time zone identifier or an offset time zone identifier.
    // 3. Return systemTimeZoneString.

    // TODO: implement this
    "UTC"
}

/// ### [21.4.1.25 LocalTime ( t )](https://tc39.es/ecma262/#sec-localtime)
///
/// The abstract operation LocalTime takes argument t (a finite time value) and
/// returns an integral Number. It converts t from UTC to local time. The local
/// political rules for standard time and daylight saving time in effect at t
/// should be used to determine the result in the way specified in this
/// section.
///
/// > #### Note 1
/// >
/// > If political rules for the local time t are not available within the
/// > implementation, the result is t because SystemTimeZoneIdentifier returns
/// > "UTC" and GetNamedTimeZoneOffsetNanoseconds returns 0.
/// >
/// > #### Note 2
/// >
/// > It is required for time zone aware implementations (and recommended for
/// > all others) to use the time zone information of the IANA Time Zone
/// > Database https://www.iana.org/time-zones/.
/// >
/// > #### Note 3
/// >
/// > Two different input time values tUTC are converted to the same local time
/// > tlocal at a negative time zone transition when there are repeated times
/// > (e.g. the daylight saving time ends or the time zone adjustment is
/// > decreased.).
/// >
/// > LocalTime(UTC(tlocal)) is not necessarily always equal to tlocal.
/// > Correspondingly, UTC(LocalTime(tUTC)) is not necessarily always equal to tUTC.
fn local_time(agent: &Agent, t: f64) -> f64 {
    // 1. Let systemTimeZoneIdentifier be SystemTimeZoneIdentifier().
    let system_time_zone_identifier = system_time_zone_identifier(agent);
    // 2. If IsTimeZoneOffsetString(systemTimeZoneIdentifier) is true, then
    let offset_ns = if is_time_zone_offset_string(system_time_zone_identifier) {
        // a. Let offsetNs be ParseTimeZoneOffsetString(systemTimeZoneIdentifier).
        parse_time_zone_offset_string(system_time_zone_identifier)
    }
    // 3. Else,
    else {
        // a. Let offsetNs be GetNamedTimeZoneOffsetNanoseconds(systemTimeZoneIdentifier, ℤ(ℝ(t) × 10**6)).
        get_named_time_zone_offset_nanoseconds(
            system_time_zone_identifier,
            (t * 1_000_000.0) as i64,
        )
    };
    // 4. Let offsetMs be truncate(offsetNs / 10**6).
    let offset_ms = (offset_ns / 1_000_000.0).trunc();
    // 5. Return t + 𝔽(offsetMs).
    t + offset_ms
}

fn local_or_utc_time<const UTC: bool>(agent: &mut Agent, t: f64) -> f64 {
    if UTC { t } else { local_time(agent, t) }
}

/// ### [21.4.1.26 UTC ( t )](https://tc39.es/ecma262/#sec-utc-t)
///
/// The abstract operation UTC takes argument t (a Number) and returns a time
/// value. It converts t from local time to a UTC time value. The local
/// political rules for standard time and daylight saving time in effect at t
/// should be used to determine the result in the way specified in this section.
///
/// Input t is nominally a time value but may be any Number value.
/// The algorithm must not limit t to the time value range, so that inputs
/// corresponding with a boundary of the time value range can be supported
/// regardless of local UTC offset. For example, the maximum time value is 8.64
/// × 10**15, corresponding with "+275760-09-13T00:00:00Z". In an environment
/// where the local time zone offset is ahead of UTC by 1 hour at that instant,
/// it is represented by the larger input of 8.64 × 10**15 + 3.6 × 10**6,
/// corresponding with "+275760-09-13T01:00:00+01:00".
///
/// If political rules for the local time t are not available within the
/// implementation, the result is t because SystemTimeZoneIdentifier returns
/// "UTC" and GetNamedTimeZoneOffsetNanoseconds returns 0.
///
/// > #### Note 1
/// >
/// > It is required for time zone aware implementations (and recommended for
/// > all others) to use the time zone information of the IANA Time Zone
/// > Database https://www.iana.org/time-zones/.
/// >
/// > 1:30 AM on 5 November 2017 in America/New_York is repeated twice (fall
/// > backward), but it must be interpreted as 1:30 AM UTC-04 instead of 1:30
/// > AM UTC-05. In UTC(TimeClip(MakeDate(MakeDay(2017, 10, 5), MakeTime(1, 30,
/// > 0, 0)))), the value of offsetMs is -4 × msPerHour.
/// >
/// > 2:30 AM on 12 March 2017 in America/New_York does not exist, but it must
/// > be interpreted as 2:30 AM UTC-05 (equivalent to 3:30 AM UTC-04). In UTC
/// > (TimeClip(MakeDate(MakeDay(2017, 2, 12), MakeTime(2, 30, 0, 0)))), the
/// > value of offsetMs is -5 × msPerHour.
/// >
/// > #### Note 2
/// >
/// > UTC(LocalTime(tUTC)) is not necessarily always equal to tUTC.
/// > Correspondingly, LocalTime(UTC(tlocal)) is not necessarily always equal
/// > to tlocal.
pub(super) fn utc(agent: &Agent, t: f64) -> f64 {
    // 1. If t is not finite, return NaN.
    if !t.is_finite() {
        return f64::NAN;
    }
    // 2. Let systemTimeZoneIdentifier be SystemTimeZoneIdentifier().
    let system_time_zone_identifier = system_time_zone_identifier(agent);
    // 3. If IsTimeZoneOffsetString(systemTimeZoneIdentifier) is true, then
    let offset_ns = if is_time_zone_offset_string(system_time_zone_identifier) {
        // a. Let offsetNs be ParseTimeZoneOffsetString(systemTimeZoneIdentifier).
        parse_time_zone_offset_string(system_time_zone_identifier)
    }
    // 4. Else,
    else {
        // a. Let possibleInstants be GetNamedTimeZoneEpochNanoseconds(systemTimeZoneIdentifier, ℝ(YearFromTime(t)), ℝ(MonthFromTime(t)) + 1, ℝ(DateFromTime(t)), ℝ(HourFromTime(t)), ℝ(MinFromTime(t)), ℝ(SecFromTime(t)), ℝ(msFromTime(t)), 0, 0).
        let possible_instants = get_named_time_zone_epoch_nanoseconds(
            system_time_zone_identifier,
            year_from_time(t),
            month_from_time(t) + 1,
            date_from_time(t),
            hour_from_time(t),
            min_from_time(t),
            sec_from_time(t),
            ms_from_time(t),
            0,
            0,
        );
        // b. NOTE: The following steps ensure that when t represents local
        // time repeating multiple times at a negative time zone transition
        // (e.g. when the daylight saving time ends or the time zone offset is
        // decreased due to a time zone rule change) or skipped local time at a
        // positive time zone transition (e.g. when the daylight saving time
        // starts or the time zone offset is increased due to a time zone rule
        // change), t is interpreted using the time zone offset before the
        // transition.
        // c. If possibleInstants is not empty, then
        let disambiguated_instant = if !possible_instants.is_empty() {
            // i. Let disambiguatedInstant be possibleInstants[0].
            possible_instants[0]
        }
        // d. Else,
        else {
            // i. NOTE: t represents a local time skipped at a positive time
            // zone transition (e.g. due to daylight saving time starting or a
            // time zone rule change increasing the UTC offset).
            // ii. Let possibleInstantsBefore be
            // GetNamedTimeZoneEpochNanoseconds(systemTimeZoneIdentifier, ℝ
            // (YearFromTime(tBefore)), ℝ(MonthFromTime(tBefore)) + 1, ℝ
            // (DateFromTime(tBefore)), ℝ(HourFromTime(tBefore)), ℝ(MinFromTime
            // (tBefore)), ℝ(SecFromTime(tBefore)), ℝ(msFromTime(tBefore)), 0,
            // 0), where tBefore is the largest integral Number < t for which
            // possibleInstantsBefore is not empty (i.e., tBefore represents
            // the last local time before the transition).
            // iii. Let disambiguatedInstant be the last element of possibleInstantsBefore.
            todo!()
        };
        // e. Let offsetNs be GetNamedTimeZoneOffsetNanoseconds(systemTimeZoneIdentifier, disambiguatedInstant).
        get_named_time_zone_offset_nanoseconds(system_time_zone_identifier, disambiguated_instant)
    };
    // 5. Let offsetMs be truncate(offsetNs / 10**6).
    let offset_ms = (offset_ns / 1_000_000.0).trunc();
    // 6. Return t - 𝔽(offsetMs).
    t - offset_ms
}

/// ### [21.4.1.27 MakeTime ( hour, min, sec, ms )](https://tc39.es/ecma262/#sec-maketime)
///
/// The abstract operation MakeTime takes arguments hour (a Number),
/// min (a Number), sec (a Number), and ms (a Number) and returns a Number.
/// It calculates a number of milliseconds.
///
/// > #### Note
/// >
/// > The arithmetic in MakeTime is floating-point arithmetic,
/// > which is not associative, so the operations must be performed in the
/// > correct order.
pub(super) fn make_time(hour: f64, min: f64, sec: f64, ms: f64) -> f64 {
    // 1. If hour is not finite, min is not finite, sec is not finite, or ms is not finite, return NaN.
    if !hour.is_finite() || !min.is_finite() || !sec.is_finite() || !ms.is_finite() {
        return f64::NAN;
    }

    // 2. Let h be 𝔽(! ToIntegerOrInfinity(hour)).
    let h = hour.abs().floor().copysign(hour);

    // 3. Let m be 𝔽(! ToIntegerOrInfinity(min)).
    let m = min.abs().floor().copysign(min);

    // 4. Let s be 𝔽(! ToIntegerOrInfinity(sec)).
    let s = sec.abs().floor().copysign(sec);

    // 5. Let milli be 𝔽(! ToIntegerOrInfinity(ms)).
    let milli = ms.abs().floor().copysign(ms);

    // 6. Return ((h × msPerHour + m × msPerMinute) + s × msPerSecond) + milli.
    ((h * MS_PER_HOUR + m * MS_PER_MINUTE) + s * MS_PER_SECOND) + milli
}

/// ### [21.4.1.28 MakeDay ( year, month, date )](https://tc39.es/ecma262/#sec-makeday)
///
/// Ported from Boa JS engine. Source https://github.com/boa-dev/boa/blob/13a030a0aa452e6f78e4a7e8bbc0e11b878bbd58/core/engine/src/builtins/date/utils.rs#L368
///
/// Copyright (c) 2019 Jason Williams
///
/// The abstract operation MakeDay takes arguments year (a Number),
/// month (a Number), and date (a Number) and returns a Number.
/// It calculates a number of days.
pub(super) fn make_day(year: f64, month: f64, date: f64) -> f64 {
    // 1. If year is not finite, month is not finite, or date is not finite, return NaN.
    if !year.is_finite() || !month.is_finite() || !date.is_finite() {
        return f64::NAN;
    }

    // 2. Let y be 𝔽(! ToIntegerOrInfinity(year)).
    let y = year.abs().floor().copysign(year);

    // 3. Let m be 𝔽(! ToIntegerOrInfinity(month)).
    let m = month.abs().floor().copysign(month);

    // 4. Let dt be 𝔽(! ToIntegerOrInfinity(date)).
    let dt = date.abs().floor().copysign(date);

    // 5. Let ym be y + 𝔽(floor(ℝ(m) / 12)).
    let ym = y + (m / 12.0).floor();

    // 6. If ym is not finite, return NaN.
    if !ym.is_finite() {
        return f64::NAN;
    }

    // 7. Let mn be 𝔽(ℝ(m) modulo 12).
    let mn = m.rem_euclid(12.0) as u8;

    // 8. Find a finite time value t such that YearFromTime(t) is ym, MonthFromTime(t) is mn,
    //    and DateFromTime(t) is 1𝔽;
    //    but if this is not possible (because some argument is out of range), return NaN.
    let rest = if mn > 1 { 1.0 } else { 0.0 };
    let days_within_year_to_end_of_month = match mn {
        0 => 0.0,
        1 => 31.0,
        2 => 59.0,
        3 => 90.0,
        4 => 120.0,
        5 => 151.0,
        6 => 181.0,
        7 => 212.0,
        8 => 243.0,
        9 => 273.0,
        10 => 304.0,
        11 => 334.0,
        12 => 365.0,
        _ => unreachable!(),
    };
    let t =
        (day_from_year(ym + rest) - 365.0 * rest + days_within_year_to_end_of_month) * MS_PER_DAY;

    // 9. Return Day(t) + dt - 1𝔽.
    day(t) + dt - 1.0
}

/// ### [21.4.1.29 MakeDate ( day, time )](https://tc39.es/ecma262/#sec-makedate)
///
/// The abstract operation MakeDate takes arguments day (a Number) and time (a Number)
/// and returns a Number. It calculates a number of milliseconds.
pub(super) fn make_date(day: f64, time: f64) -> f64 {
    // 1. If day is not finite or time is not finite, return NaN.
    if !day.is_finite() || !time.is_finite() {
        return f64::NAN;
    }

    // 2. Let tv be day × msPerDay + time.
    let tv = day * MS_PER_DAY + time;

    // 3. If tv is not finite, return NaN.
    if !tv.is_finite() {
        return f64::NAN;
    }

    // 4. Return tv.
    tv
}

/// ### [21.4.1.30 MakeFullYear ( year )](https://tc39.es/ecma262/#sec-makefullyear)
///
/// The abstract operation MakeFullYear takes argument year (a Number) and
/// returns an integral Number or NaN. It returns the full year associated with
/// the integer part of year, interpreting any value in the inclusive interval
/// from 0 to 99 as a count of years since the start of 1900. For alignment
/// with the proleptic Gregorian calendar, "full year" is defined as the signed
/// count of complete years since the start of year 0 (1 B.C.). It performs the
/// following steps when called:
pub(super) fn make_full_year(year: f64) -> f64 {
    // 1. If year is NaN, return NaN.
    if year.is_nan() {
        return f64::NAN;
    }

    // 2. Let truncated be ! ToIntegerOrInfinity(year).
    let truncated = to_integer_or_infinity_f64(year);

    // 3. If truncated is in the inclusive interval from 0 to 99, return 1900𝔽 + 𝔽(truncated).
    if let 0.0..=99.0 = truncated {
        return 1900.0 + truncated;
    }

    // 4. Return 𝔽(truncated).
    truncated
}

/// ### [21.4.1.33.1 IsTimeZoneOffsetString ( offsetString )](https://tc39.es/ecma262/#sec-istimezoneoffsetstring)
///
/// The abstract operation IsTimeZoneOffsetString takes argument offsetString
/// (a String) and returns a Boolean. The return value indicates whether
/// offsetString conforms to the grammar given by UTCOffset. It performs the
/// following steps when called:
fn is_time_zone_offset_string(_offset_string: &str) -> bool {
    // 1. Let parseResult be ParseText(offsetString, UTCOffset).
    let parse_result: Result<(), ()> = Err(()); // TODO:
    // 2. If parseResult is a List of errors, return false.
    if parse_result.is_err() {
        return false;
    }
    // 3. Return true.
    true
}

/// ### [21.4.1.33.2 ParseTimeZoneOffsetString ( offsetString )](https://tc39.es/ecma262/#sec-parsetimezoneoffsetstring)
///
/// The abstract operation ParseTimeZoneOffsetString takes argument
/// offsetString (a String) and returns an integer. The return value is the UTC
/// offset, as a number of nanoseconds, that corresponds to the String
/// offsetString.
fn parse_time_zone_offset_string(_offset_string: &str) -> f64 {
    // 1. Let parseResult be ParseText(offsetString, UTCOffset).
    // 2. Assert: parseResult is not a List of errors.
    // 3. Assert: parseResult contains a ASCIISign Parse Node.
    // 4. Let parsedSign be the source text matched by the ASCIISign Parse Node contained within parseResult.
    // 5. If parsedSign is the single code point U+002D (HYPHEN-MINUS), then
    //    a. Let sign be -1.
    // 6. Else,
    //    a. Let sign be 1.
    // 7. NOTE: Applications of StringToNumber below do not lose precision, since each of the parsed values is guaranteed to be a sufficiently short string of decimal digits.
    // 8. Assert: parseResult contains an Hour Parse Node.
    // 9. Let parsedHours be the source text matched by the Hour Parse Node contained within parseResult.
    // 10. Let hours be ℝ(StringToNumber(CodePointsToString(parsedHours))).
    // 11. If parseResult does not contain a MinuteSecond Parse Node, then
    //     a. Let minutes be 0.
    // 12. Else,
    //     a. Let parsedMinutes be the source text matched by the first MinuteSecond Parse Node contained within parseResult.
    //     b. Let minutes be ℝ(StringToNumber(CodePointsToString(parsedMinutes))).
    // 13. If parseResult does not contain two MinuteSecond Parse Nodes, then
    //     a. Let seconds be 0.
    // 14. Else,
    //     a. Let parsedSeconds be the source text matched by the second MinuteSecond Parse Node contained within parseResult.
    //     b. Let seconds be ℝ(StringToNumber(CodePointsToString(parsedSeconds))).
    // 15. If parseResult does not contain a TemporalDecimalFraction Parse Node, then
    //     a. Let nanoseconds be 0.
    // 16. Else,
    //     a. Let parsedFraction be the source text matched by the TemporalDecimalFraction Parse Node contained within parseResult.
    //     b. Let fraction be the string-concatenation of CodePointsToString(parsedFraction) and "000000000".
    //     c. Let nanosecondsString be the substring of fraction from 1 to 10.
    //     d. Let nanoseconds be ℝ(StringToNumber(nanosecondsString)).
    // 17. Return sign × (((hours × 60 + minutes) × 60 + seconds) × 10**9 + nanoseconds).
    todo!()
}

/// ### [21.4.4.41.1 TimeString ( tv )](https://tc39.es/ecma262/#sec-timestring)
///
/// The abstract operation TimeString takes argument tv (a Number, but not NaN)
/// and returns a String.
fn time_string(tv: f64) -> std::string::String {
    // 1. Let hour be ToZeroPaddedDecimalString(ℝ(HourFromTime(tv)), 2).
    let hour = to_zero_padded_decimal_string(hour_from_time(tv), 2);
    // 2. Let minute be ToZeroPaddedDecimalString(ℝ(MinFromTime(tv)), 2).
    let minute = to_zero_padded_decimal_string(min_from_time(tv), 2);
    // 3. Let second be ToZeroPaddedDecimalString(ℝ(SecFromTime(tv)), 2).
    let second = to_zero_padded_decimal_string(sec_from_time(tv), 2);
    // 4. Return the string-concatenation of hour, ":", minute, ":", second, the code unit 0x0020 (SPACE), and "GMT".
    format!("{hour}:{minute}:{second} GMT")
}

/// ### [21.4.4.41.2 DateString ( tv )](https://tc39.es/ecma262/#sec-datestring)
///
/// The abstract operation DateString takes argument tv (a Number, but not NaN)
/// and returns a String.
fn date_string(tv: f64) -> std::string::String {
    // 1. Let weekday be the Name of the entry in Table 65 with the Number WeekDay(tv).
    let weekday = match week_day(tv) {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        _ => unreachable!(),
    };
    // 2. Let month be the Name of the entry in Table 66 with the Number MonthFromTime(tv).
    let month = match month_from_time(tv) {
        0 => "Jan",
        1 => "Feb",
        2 => "Mar",
        3 => "Apr",
        4 => "May",
        5 => "Jun",
        6 => "Jul",
        7 => "Aug",
        8 => "Sep",
        9 => "Oct",
        10 => "Nov",
        11 => "Dec",
        _ => unreachable!(),
    };
    // 3. Let day be ToZeroPaddedDecimalString(ℝ(DateFromTime(tv)), 2).
    let day = to_zero_padded_decimal_string(date_from_time(tv), 2);
    // 4. Let yv be YearFromTime(tv).
    let yv = year_from_time(tv);
    // 5. If yv is +0𝔽 or yv > +0𝔽, let yearSign be the empty String; otherwise, let yearSign be "-".
    let year_sign = if yv >= 0 { "" } else { "-" };
    // 6. Let paddedYear be ToZeroPaddedDecimalString(abs(ℝ(yv)), 4).
    let padded_year = to_zero_padded_decimal_string(yv.abs(), 4);
    // 7. Return the string-concatenation of weekday, the code unit 0x0020 (SPACE), month,
    // the code unit 0x0020 (SPACE), day, the code unit 0x0020 (SPACE), yearSign, and paddedYear.
    format!("{weekday} {month} {day} {year_sign}{padded_year}")
}

/// ### [21.4.4.41.3 TimeZoneString ( tv )](https://tc39.es/ecma262/#sec-timezoneestring)
///
/// The abstract operation TimeZoneString takes argument tv (an integral
/// Number) and returns a String.
fn time_zone_string(agent: &Agent, tv: f64) -> std::string::String {
    // 1. Let systemTimeZoneIdentifier be SystemTimeZoneIdentifier().
    let system_time_zone_identifier = system_time_zone_identifier(agent);
    // 2. If IsTimeZoneOffsetString(systemTimeZoneIdentifier) is true, then
    let offset_ns = if is_time_zone_offset_string(system_time_zone_identifier) {
        // a. Let offsetNs be ParseTimeZoneOffsetString(systemTimeZoneIdentifier).
        parse_time_zone_offset_string(system_time_zone_identifier)
    }
    // 3. Else,
    else {
        // a. Let offsetNs be GetNamedTimeZoneOffsetNanoseconds(systemTimeZoneIdentifier, ℤ(ℝ(tv) × 10**6)).
        get_named_time_zone_offset_nanoseconds(
            system_time_zone_identifier,
            (tv * 1_000_000.0) as i64,
        )
    };
    // 4. Let offset be 𝔽(truncate(offsetNs / 10**6)).
    let offset = (offset_ns / 1_000_000.0).trunc();
    // 5. If offset is +0𝔽 or offset > +0𝔽, then
    let (offset_sign, abs_offset) = if offset >= 0.0 {
        // a. Let offsetSign be "+".
        // b. Let absOffset be offset.
        ("+", offset)
    }
    // 6. Else,
    else {
        // a. Let offsetSign be "-".
        // b. Let absOffset be -offset.
        ("-", -offset)
    };
    // 7. Let offsetMin be ToZeroPaddedDecimalString(ℝ(MinFromTime(absOffset)), 2).
    let offset_min = to_zero_padded_decimal_string(min_from_time(abs_offset), 2);
    // 8. Let offsetHour be ToZeroPaddedDecimalString(ℝ(HourFromTime(absOffset)), 2).
    let offset_hour = to_zero_padded_decimal_string(hour_from_time(abs_offset), 2);
    // 9. Let tzName be an implementation-defined string that is either the
    // empty String or the string-concatenation of the code unit 0x0020
    // (SPACE), the code unit 0x0028 (LEFT PARENTHESIS), an
    // implementation-defined timezone name, and the code unit 0x0029 (RIGHT
    // PARENTHESIS).
    let tz_name = "";
    // 10. Return the string-concatenation of offsetSign, offsetHour, offsetMin, and tzName.
    format!("{offset_sign}{offset_hour}{offset_min}{tz_name}")
}

/// ### [21.4.4.41.4 ToDateString ( tv )](https://tc39.es/ecma262/#sec-todatestring)
///
/// The abstract operation ToDateString takes argument tv (an integral Number
/// or NaN) and returns a String.
pub(super) fn to_date_string(agent: &Agent, tv: DateValue) -> std::string::String {
    // 1. If tv is NaN, return "Invalid Date".
    let Some(tv) = tv.get_f64() else {
        return "Invalid Date".to_string();
    };
    // 2. Let t be LocalTime(tv).
    let t = local_time(agent, tv);
    // 3. Return the string-concatenation of DateString(t), the code unit 0x0020 (SPACE), TimeString(t), and TimeZoneString(tv).
    format!(
        "{} {}{}",
        date_string(t),
        time_string(t),
        time_zone_string(agent, tv)
    )
}
