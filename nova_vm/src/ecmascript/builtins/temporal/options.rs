// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{num::NonZeroU32, str::FromStr};

use temporal_rs::options::{RoundingIncrement, RoundingMode, Unit};

use crate::{
    ecmascript::{
        Agent, BUILTIN_STRING_MEMORY, ExceptionType, JsResult, Object, PropertyKey, Value, get,
        ordinary_object_create_null, to_integer_with_truncation, to_string,
    },
    engine::{Bindable, GcScope, NoGcScope},
};

pub trait OptionType: Sized {
    fn from_string(s: &str) -> Result<Self, std::string::String>;
}

impl OptionType for RoundingMode {
    fn from_string(s: &str) -> Result<Self, std::string::String> {
        RoundingMode::from_str(s).map_err(|e| e.to_string())
    }
}

impl OptionType for Unit {
    fn from_string(s: &str) -> Result<Self, std::string::String> {
        Unit::from_str(s).map_err(|e| e.to_string())
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
) -> JsResult<'gc, Object<'gc>> {
    match options.unbind() {
        // 1. If options is undefined, then
        Value::Undefined => {
            // a. Return OrdinaryObjectCreate(null).
            Ok(ordinary_object_create_null(agent, gc).into())
        }
        // 2. If options is an Object, then
        value if value.is_object() => {
            // a. Return options.
            // TODO: remove unwrap; Although safe because value is an object
            let obj = Object::try_from(value).unwrap();
            Ok(obj)
        }
        // 3. Throw a TypeError exception.
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "options provided to GetOptionsObject is not an object",
            gc,
        )),
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
    // 1. Let value be ? Get(options, property).
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

    // TODO: Currently only works for temporal_rs::Unit, and temporal_rs::RoundingMode.
    //
    // Should be extended to work with
    // 1. ecmascript::types::String
    // 2. bool
    // 3. Potentially other temporal_rs types.

    let js_str = to_string(agent, value.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // TODO: Fix this code.. None case is unreachable but code sucks rn..
    let rust_str = js_str.as_str(agent).unwrap();

    let parsed = T::from_string(rust_str).map_err(|msg| {
        agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            Box::leak(msg.into_boxed_str()),
            gc.into_nogc(),
        )
    })?;

    // 6. Return value.
    Ok(Some(parsed))
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
    let fallback = fallback.bind(gc.nogc());

    // 1. Let allowedStrings be the List of Strings from the "String Identifier" column of Table 28.
    // 2. Let stringFallback be the value from the "String Identifier" column of the row with fallback in its "Rounding Mode" column.
    // 3. Let stringValue be ? GetOption(options, "roundingMode", string, allowedStrings, stringFallback).
    // 4. Return the value from the "Rounding Mode" column of the row with stringValue in its "String Identifier" column.
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
/// The abstract operation GetRoundingIncrementOption takes argument options (an Object) and returns
/// either a normal completion containing a positive integer in the inclusive interval from 1 to
/// 10**9, or a throw completion. It fetches and validates the "roundingIncrement" property from
/// options, returning a default if absent. It performs the following steps when called:
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
    .unbind()?;
    // 2. If value is undefined, return 1𝔽.
    if value.is_undefined() {
        return Ok(RoundingIncrement::default());
    }
    // 3. Let integerIncrement be ? ToIntegerWithTruncation(value).
    let integer_increment = to_integer_with_truncation(agent, value, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // TODO(jesper): https://github.com/trynova/nova/pull/876#discussion_r2611571860

    // 4. If integerIncrement < 1 or integerIncrement > 10**9, throw a RangeError exception.
    if !(1.0..=1_000_000_000.0).contains(&integer_increment) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "roundingIncrement must be between 1 and 10**9",
            gc.into_nogc(),
        ));
    }

    // NOTE: `as u32` is safe here since we validated it’s in range.
    let integer_increment_u32 = integer_increment as u32;
    let increment =
        NonZeroU32::new(integer_increment_u32).expect("integer_increment >= 1 ensures nonzero");

    // 5. Return integerIncrement.
    Ok(RoundingIncrement::new_unchecked(increment))
}
