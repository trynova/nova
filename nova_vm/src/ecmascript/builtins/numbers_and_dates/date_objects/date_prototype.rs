use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct DatePrototype;

struct DatePrototypeGetDate;
impl Builtin for DatePrototypeGetDate {
    const NAME: String = BUILTIN_STRING_MEMORY.getDate;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_date);
}
struct DatePrototypeGetDay;
impl Builtin for DatePrototypeGetDay {
    const NAME: String = BUILTIN_STRING_MEMORY.getDay;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_day);
}
struct DatePrototypeGetFullYear;
impl Builtin for DatePrototypeGetFullYear {
    const NAME: String = BUILTIN_STRING_MEMORY.getFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_full_year);
}
struct DatePrototypeGetHours;
impl Builtin for DatePrototypeGetHours {
    const NAME: String = BUILTIN_STRING_MEMORY.getHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_hours);
}
struct DatePrototypeGetMilliseconds;
impl Builtin for DatePrototypeGetMilliseconds {
    const NAME: String = BUILTIN_STRING_MEMORY.getMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_milliseconds);
}
struct DatePrototypeGetMinutes;
impl Builtin for DatePrototypeGetMinutes {
    const NAME: String = BUILTIN_STRING_MEMORY.getMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_minutes);
}
struct DatePrototypeGetMonth;
impl Builtin for DatePrototypeGetMonth {
    const NAME: String = BUILTIN_STRING_MEMORY.getMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_month);
}
struct DatePrototypeGetSeconds;
impl Builtin for DatePrototypeGetSeconds {
    const NAME: String = BUILTIN_STRING_MEMORY.getSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_seconds);
}
struct DatePrototypeGetTime;
impl Builtin for DatePrototypeGetTime {
    const NAME: String = BUILTIN_STRING_MEMORY.getTime;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_time);
}
struct DatePrototypeGetTimezoneOffset;
impl Builtin for DatePrototypeGetTimezoneOffset {
    const NAME: String = BUILTIN_STRING_MEMORY.getTimezoneOffset;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_timezone_offset);
}
struct DatePrototypeGetUTCDate;
impl Builtin for DatePrototypeGetUTCDate {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCDate;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_date);
}
struct DatePrototypeGetUTCDay;
impl Builtin for DatePrototypeGetUTCDay {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCDay;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_day);
}
struct DatePrototypeGetUTCFullYear;
impl Builtin for DatePrototypeGetUTCFullYear {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_full_year);
}
struct DatePrototypeGetUTCHours;
impl Builtin for DatePrototypeGetUTCHours {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_hours);
}
struct DatePrototypeGetUTCMilliseconds;
impl Builtin for DatePrototypeGetUTCMilliseconds {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_milliseconds);
}
struct DatePrototypeGetUTCMinutes;
impl Builtin for DatePrototypeGetUTCMinutes {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_minutes);
}
struct DatePrototypeGetUTCMonth;
impl Builtin for DatePrototypeGetUTCMonth {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_month);
}
struct DatePrototypeGetUTCSeconds;
impl Builtin for DatePrototypeGetUTCSeconds {
    const NAME: String = BUILTIN_STRING_MEMORY.getUTCSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::get_utc_seconds);
}
struct DatePrototypeSetDate;
impl Builtin for DatePrototypeSetDate {
    const NAME: String = BUILTIN_STRING_MEMORY.setDate;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_date);
}
struct DatePrototypeSetFullYear;
impl Builtin for DatePrototypeSetFullYear {
    const NAME: String = BUILTIN_STRING_MEMORY.setFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_full_year);
}
struct DatePrototypeSetHours;
impl Builtin for DatePrototypeSetHours {
    const NAME: String = BUILTIN_STRING_MEMORY.setHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_hours);
}
struct DatePrototypeSetMilliseconds;
impl Builtin for DatePrototypeSetMilliseconds {
    const NAME: String = BUILTIN_STRING_MEMORY.setMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_milliseconds);
}
struct DatePrototypeSetMinutes;
impl Builtin for DatePrototypeSetMinutes {
    const NAME: String = BUILTIN_STRING_MEMORY.setMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_minutes);
}
struct DatePrototypeSetMonth;
impl Builtin for DatePrototypeSetMonth {
    const NAME: String = BUILTIN_STRING_MEMORY.setMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_month);
}
struct DatePrototypeSetSeconds;
impl Builtin for DatePrototypeSetSeconds {
    const NAME: String = BUILTIN_STRING_MEMORY.setSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_seconds);
}
struct DatePrototypeSetTime;
impl Builtin for DatePrototypeSetTime {
    const NAME: String = BUILTIN_STRING_MEMORY.setTime;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_time);
}
struct DatePrototypeSetUTCDate;
impl Builtin for DatePrototypeSetUTCDate {
    const NAME: String = BUILTIN_STRING_MEMORY.setUTCDate;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_date);
}
struct DatePrototypeSetUTCFullYear;
impl Builtin for DatePrototypeSetUTCFullYear {
    const NAME: String = BUILTIN_STRING_MEMORY.setUTCFullYear;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_full_year);
}
struct DatePrototypeSetUTCHours;
impl Builtin for DatePrototypeSetUTCHours {
    const NAME: String = BUILTIN_STRING_MEMORY.setUTCHours;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_hours);
}
struct DatePrototypeSetUTCMilliseconds;
impl Builtin for DatePrototypeSetUTCMilliseconds {
    const NAME: String = BUILTIN_STRING_MEMORY.setUTCMilliseconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_milliseconds);
}
struct DatePrototypeSetUTCMinutes;
impl Builtin for DatePrototypeSetUTCMinutes {
    const NAME: String = BUILTIN_STRING_MEMORY.setUTCMinutes;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_minutes);
}
struct DatePrototypeSetUTCMonth;
impl Builtin for DatePrototypeSetUTCMonth {
    const NAME: String = BUILTIN_STRING_MEMORY.setUTCMonth;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_month);
}
struct DatePrototypeSetUTCSeconds;
impl Builtin for DatePrototypeSetUTCSeconds {
    const NAME: String = BUILTIN_STRING_MEMORY.setUTCSeconds;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::set_utc_seconds);
}
struct DatePrototypeToDateString;
impl Builtin for DatePrototypeToDateString {
    const NAME: String = BUILTIN_STRING_MEMORY.toDateString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_date_string);
}
struct DatePrototypeToISOString;
impl Builtin for DatePrototypeToISOString {
    const NAME: String = BUILTIN_STRING_MEMORY.toISOString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_iso_string);
}
struct DatePrototypeToJSON;
impl Builtin for DatePrototypeToJSON {
    const NAME: String = BUILTIN_STRING_MEMORY.toJSON;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_json);
}
struct DatePrototypeToLocaleDateString;
impl Builtin for DatePrototypeToLocaleDateString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleDateString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_locale_date_string);
}
struct DatePrototypeToLocaleString;
impl Builtin for DatePrototypeToLocaleString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_locale_string);
}
struct DatePrototypeToLocaleTimeString;
impl Builtin for DatePrototypeToLocaleTimeString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleTimeString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_locale_time_string);
}
struct DatePrototypeToString;
impl Builtin for DatePrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_string);
}
struct DatePrototypeToTimeString;
impl Builtin for DatePrototypeToTimeString {
    const NAME: String = BUILTIN_STRING_MEMORY.toTimeString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_time_string);
}
struct DatePrototypeToUTCString;
impl Builtin for DatePrototypeToUTCString {
    const NAME: String = BUILTIN_STRING_MEMORY.toUTCString;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::to_utc_string);
}
struct DatePrototypeValueOf;
impl Builtin for DatePrototypeValueOf {
    const NAME: String = BUILTIN_STRING_MEMORY.valueOf;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DatePrototype::value_of);
}
struct DatePrototypeToPrimitive;
impl Builtin for DatePrototypeToPrimitive {
    const NAME: String = BUILTIN_STRING_MEMORY._Symbol_toPrimitive_;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(DatePrototype::to_primitive);
}

impl DatePrototype {
    fn get_date(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_day(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_full_year(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_hours(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_milliseconds(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_minutes(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_month(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_seconds(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_time(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_timezone_offset(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_date(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_day(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_full_year(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_hours(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_milliseconds(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_minutes(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_month(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_utc_seconds(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set_date(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_full_year(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_hours(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_milliseconds(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set_minutes(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_month(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_seconds(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_time(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_utc_date(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_utc_full_year(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set_utc_hours(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_utc_milliseconds(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set_utc_minutes(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set_utc_month(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_utc_seconds(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn to_date_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_iso_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_json(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_locale_date_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn to_locale_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn to_locale_time_string(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn to_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_time_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_utc_string(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn value_of(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn to_primitive(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.date_prototype();
        let date_constructor = intrinsics.date();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(7)
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
            .with_builtin_function_property::<DatePrototypeToUTCString>()
            .with_builtin_function_property::<DatePrototypeValueOf>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToPrimitive.into())
                    .with_value_creator_readonly(|agent| {
                        BuiltinFunctionBuilder::new::<DatePrototypeToPrimitive>(agent, realm)
                            .build()
                            .into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

#[inline(always)]
fn check_date_object(agent: &mut Agent, this_value: Value) -> JsResult<Object> {
    match this_value {
        Value::Date(idx) => Ok(Object::Date(idx)),
        _ => Err(agent.throw_exception(ExceptionType::TypeError, "this is not a Date object.")),
    }
}

/// ### [21.1.3.7.1 ThisNumberValue ( value )](https://tc39.es/ecma262/#sec-thisnumbervalue)
///
/// The abstract operation ThisNumberValue takes argument value (an ECMAScript language value) and returns either a normal completion containing a Number or a throw completion. It performs the following steps when called:
#[inline(always)]
fn this_number_value(agent: &mut Agent, value: Value) -> JsResult<Number> {
    // 1. If value is a Number, return value.
    if let Ok(value) = Number::try_from(value) {
        return Ok(value);
    }
    // 2. If value is an Object and value has a [[NumberData]] internal slot, then
    // a. Let n be value.[[NumberData]].
    // b. Assert: n is a Number.
    // c. Return n.
    // 3. Throw a TypeError exception.
    Err(agent.throw_exception(ExceptionType::TypeError, "Not a Number"))
}
