// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::str::FromStr;

use temporal_rs::options::{RoundingIncrement, RoundingMode, Unit};

use crate::{
    ecmascript::{
        Agent, BUILTIN_STRING_MEMORY, ExceptionType, JsResult, Object, PropertyKey, String, Value,
        get, temporal_err_to_js_err, to_integer_with_truncation, to_string,
    },
    engine::{Bindable, GcScope, NoGcScope},
};

pub(crate) trait OptionType: Sized {
    fn from_value<'gc>(
        agent: &mut Agent,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Self>;
}

pub(crate) trait StringOptionType: Sized {
    fn from_string<'gc>(
        agent: &mut Agent,
        value: String,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, Self>;
}

impl<T: StringOptionType> OptionType for T {
    fn from_value<'gc>(
        agent: &mut Agent,
        value: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Self> {
        // b. Set value to ? ToString(value).
        let value = to_string(agent, value, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        Self::from_string(agent, value.unbind(), gc.into_nogc())
    }
}

impl StringOptionType for RoundingMode {
    fn from_string<'gc>(
        agent: &mut Agent,
        value: String,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, Self> {
        let value = value.as_str(agent).unwrap_or("");
        Self::from_str(value).map_err(|err| temporal_err_to_js_err(agent, err, gc.into_nogc()))
    }
}

impl StringOptionType for Unit {
    fn from_string<'gc>(
        agent: &mut Agent,
        value: String,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, Self> {
        let value = value.as_str(agent).unwrap_or("");
        Self::from_str(value).map_err(|err| {
            agent.throw_exception(ExceptionType::RangeError, format!("{err}"), gc.into_nogc())
        })
    }
}

/// ### [14.5.2.1 GetOptionsObject ( options )](https://tc39.es/proposal-temporal/#sec-getoptionsobject)
///
/// The abstract operation GetOptionsObject takes argument options (an ECMAScript language value)
/// and returns either a normal completion containing an Object or a throw completion. It returns
/// an Object suitable for use with GetOption, either options itself or a default empty Object. It
/// throws a TypeError if options is not undefined and not an Object. It performs the following
/// steps when called:
pub(crate) fn get_options_object<'gc>(
    agent: &mut Agent,
    options: Value,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, Option<Object<'gc>>> {
    let options = options.bind(gc);
    // 1. If options is undefined, then
    if options.is_undefined() {
        // a. Return OrdinaryObjectCreate(null).
        Ok(None)
    } else if let Ok(options) = Object::try_from(options) {
        // 2. If options is an Object, then
        // a. Return options.
        Ok(Some(options))
    } else {
        // 3. Throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "options provided to GetOptionsObject is not an object",
            gc,
        ))
    }
}

/// ### [14.5.2.2 GetOption ( options, property, type, values, default )](https://tc39.es/proposal-temporal/#sec-getoption)
///
/// The abstract operation GetOption takes arguments options (an Object), property (a property
/// key), type (boolean or string), values (empty or a List of ECMAScript language values), and
/// default (required or an ECMAScript language value) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. It extracts the value of the
/// specified property of options, converts it to the required type, checks whether it is allowed
/// by values if values is not empty, and substitutes default if the value is undefined. It
/// performs the following steps when called:
pub(crate) fn get_option<'gc, T>(
    agent: &mut Agent,
    options: Object,
    property: PropertyKey,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Option<T>>
where
    T: OptionType,
{
    let options = options.bind(gc.nogc());
    let property = property.bind(gc.nogc());
    // 1. Let value be ? Get(options, property).
    let value = get(agent, options.unbind(), property.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    // 2. If value is undefined, then
    if value.is_undefined() {
        // a. If default is required, throw a RangeError exception.
        // b. Return default.
        return Ok(None);
    }

    // 3. If type is boolean, then
    // a. Set value to ToBoolean(value).
    // 4. Else,
    // a. Assert: type is string.
    // b. Set value to ? ToString(value).
    // 5. If values is not empty and values does not contain value, throw a RangeError exception.
    // 6. Return value.
    T::from_value(agent, value.unbind(), gc).map(Some)
}

/// ### [14.5.2.3 GetRoundingModeOption ( options, fallback)](https://tc39.es/proposal-temporal/#sec-temporal-getroundingmodeoption)
///
// The abstract operation GetRoundingModeOption takes arguments options (an Object) and fallback (a
// rounding mode) and returns either a normal completion containing a rounding mode, or a throw
// completion. It fetches and validates the "roundingMode" property from options, returning
// fallback as a default if absent. It performs the following steps when called:
pub(crate) fn get_rounding_mode_option<'gc>(
    agent: &mut Agent,
    options: Object,
    fallback: RoundingMode,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, RoundingMode> {
    let options = options.bind(gc.nogc());

    // 1. Let allowedStrings be the List of Strings from the "String Identifier"
    // column of Table 28.
    // 2. Let stringFallback be the value from the "String Identifier" column of
    // the row with fallback in its "Rounding Mode" column.
    // 3. Let stringValue be ? GetOption(options, "roundingMode", string,
    // allowedStrings, stringFallback).
    // 4. Return the value from the "Rounding Mode" column of the row with
    // stringValue in its "String Identifier" column.
    match get_option::<RoundingMode>(
        agent,
        options.unbind(),
        BUILTIN_STRING_MEMORY.roundingMode.into(),
        gc,
    )? {
        Some(mode) => Ok(mode),
        None => Ok(fallback),
    }
}

/// ### [14.5.2.4 GetRoundingIncrementOption ( options )](https://tc39.es/proposal-temporal/#sec-temporal-getroundingincrementoption)
///
/// The abstract operation GetRoundingIncrementOption takes argument options (an
/// Object) and returns either a normal completion containing a positive integer
/// in the inclusive interval from 1 to 10**9, or a throw completion. It fetches
/// and validates the "roundingIncrement" property from options, returning a
/// default if absent. It performs the following steps when called:
pub(crate) fn get_rounding_increment_option<'gc>(
    agent: &mut Agent,
    options: Object,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, RoundingIncrement> {
    let options = options.bind(gc.nogc());
    // 1. Let value be ? Get(options, "roundingIncrement").
    let value = get(
        agent,
        options.unbind(),
        BUILTIN_STRING_MEMORY.roundingIncrement.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 2. If value is undefined, return 1𝔽.
    if value.is_undefined() {
        return Ok(RoundingIncrement::default());
    }
    // 3. Let integerIncrement be ? ToIntegerWithTruncation(value).
    let integer_increment =
        to_integer_with_truncation(agent, value.unbind(), gc.reborrow()).unbind()?;

    // 4. If integerIncrement < 1 or integerIncrement > 10**9, throw a
    // RangeError exception.
    // 5. Return integerIncrement.
    RoundingIncrement::try_new(u32::try_from(integer_increment).unwrap_or(u32::MAX))
        .map_err(|err| temporal_err_to_js_err(agent, err, gc.into_nogc()))
}
