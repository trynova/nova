// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{num::NonZeroU32, str::FromStr};

use temporal_rs::options::{RoundingIncrement, RoundingMode};

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::get,
            type_conversion::{to_boolean, to_string},
        },
        builtins::{
            ordinary::ordinary_object_create_with_intrinsics,
            temporal::{
                error::temporal_err_to_js_err,
                instant::instant_prototype::to_integer_with_truncation,
            },
        },
        execution::{Agent, JsResult, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, Object, PropertyKey, Value},
    },
    engine::context::{Bindable, GcScope, NoGcScope, trivially_bindable},
};

#[derive(Debug)]
pub(crate) enum OptionType {
    Boolean,
    String,
}

trivially_bindable!(OptionType);
trivially_bindable!(RoundingMode);
trivially_bindable!(RoundingIncrement);

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
            Ok(ordinary_object_create_with_intrinsics(
                agent, None, None, gc,
            ))
        }
        // 2. If options is an Object, then
        Value::Object(obj) => {
            // a. Return options.
            Ok(obj.into())
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
pub(crate) fn get_option<'gc>(
    agent: &mut Agent,
    options: Object,
    property: PropertyKey,
    type_: OptionType,
    values: &[&str],
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
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
        todo!()
    }

    let value = match type_ {
        // 3. If type is boolean, then
        OptionType::Boolean => {
            // a. Set value to ToBoolean(value).
            Value::from(to_boolean(agent, value))
        }
        // 4. Else,
        OptionType::String => {
            // a. Assert: type is string.
            // b. Set value to ? ToString(value).
            let str = to_string(agent, value.unbind(), gc.reborrow()).unbind()?;
            str.into_value()
        }
    };

    let gc = gc.into_nogc();
    // 5. If values is not empty and values does not contain value, throw a RangeError exception.
    if !values.is_empty() {
        dbg!(value);
        let str = match value.unbind() {
            Value::SmallString(s) => s,
            _ => unreachable!(),
        };
        let rust_str = unsafe { str.as_str_unchecked() };
        if !values.contains(&rust_str) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Invalid option value",
                gc,
            ));
        }
    }

    // 6. Return value.
    Ok(value.bind(gc))
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
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, RoundingMode> {
    let options = options.bind(gc.nogc());
    let fallback = fallback.bind(gc.nogc());
    // 1. Let allowedStrings be the List of Strings from the "String Identifier" column of Table 28.
    const ALLOWED: &[&str] = &[
        "ceil",
        "floor",
        "trunc",
        "halfCeil",
        "halfFloor",
        "halfTrunc",
        "halfExpand",
    ];
    // 2. Let stringFallback be the value from the "String Identifier" column of the row with fallback in its "Rounding Mode" column.
    let string_fallback = fallback.unbind().to_string();
    // 3. Let stringValue be ? GetOption(options, "roundingMode", string, allowedStrings, stringFallback).
    let string_value = get_option(
        agent,
        options.unbind(),
        BUILTIN_STRING_MEMORY.roundingMode.into(),
        OptionType::String,
        ALLOWED,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());

    let js_str = string_value
        .unbind()
        .to_string(agent, gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    let rust_str = js_str.as_str(agent).expect("aaa");

    // 4. Return the value from the "Rounding Mode" column of the row with stringValue in its "String Identifier" column.
    RoundingMode::from_str(rust_str).map_err(|e| temporal_err_to_js_err(agent, e, gc.into_nogc()))
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

    // 4. If integerIncrement < 1 or integerIncrement > 10**9, throw a RangeError exception.
    if integer_increment < 1.0 || integer_increment > 1_000_000_000.0 {
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
