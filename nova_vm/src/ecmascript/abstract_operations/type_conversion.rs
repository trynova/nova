// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [7.1 Type Conversion](https://tc39.es/ecma262/#sec-type-conversion)
//!
//! The ECMAScript language implicitly performs automatic type conversion as
//! needed. To clarify the semantics of certain constructs it is useful to
//! define a set of conversion abstract operations. The conversion abstract
//! operations are polymorphic; they can accept a value of any ECMAScript
//! language type. But no other specification types are used with these
//! operations.
//!
//! The BigInt type has no implicit conversions in the ECMAScript language;
//! programmers must call BigInt explicitly to convert values from other types.

use num_bigint::Sign;

use crate::ecmascript::types::IntoPrimitive;
use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::TryResult;
use crate::{
    ecmascript::{
        builtins::{
            primitive_objects::{PrimitiveObjectData, PrimitiveObjectHeapData},
            ArgumentsList,
        },
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{
            BigInt, IntoNumeric, IntoObject, IntoValue, Number, Numeric, Object, Primitive,
            PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{CreateHeapData, WellKnownSymbolIndexes},
    SmallInteger,
};

use super::{
    operations_on_objects::{call_function, get, get_method},
    testing_and_comparison::is_callable,
};

#[derive(Debug, Clone, Copy)]
pub enum PreferredType {
    String = 1,
    Number,
}

/// ### [7.1.1 ToPrimitive ( input \[ , preferredType \] )](https://tc39.es/ecma262/#sec-toprimitive)
///
/// The abstract operation ToPrimitive takes argument input (an ECMAScript
/// language value) and optional argument preferredType (STRING or NUMBER) and
/// returns either a normal completion containing an ECMAScript language value
/// or a throw completion. It converts its input argument to a non-Object type.
/// If an object is capable of converting to more than one primitive type, it
/// may use the optional hint preferredType to favour that type. It performs
/// the following steps when called:
///
/// > NOTE: When ToPrimitive is called without a hint, then it generally
/// > behaves as if the hint were NUMBER. However, objects may over-ride this
/// > behaviour by defining a @@toPrimitive method. Of the objects defined in
/// > this specification only Dates (see 21.4.4.45) and Symbol objects (see
/// > 20.4.3.5) over-ride the default ToPrimitive behaviour. Dates treat the
/// > absence of a hint as if the hint were STRING.
pub(crate) fn to_primitive<'a, 'gc>(
    agent: &mut Agent,
    input: impl IntoValue<'a>,
    preferred_type: Option<PreferredType>,
    gc: GcScope<'gc, '_>,
) -> JsResult<Primitive<'gc>> {
    let input = input.into_value().bind(gc.nogc());
    // 1. If input is an Object, then
    if let Ok(input) = Object::try_from(input) {
        to_primitive_object(agent, input.unbind(), preferred_type, gc)
    } else {
        // 2. Return input.
        Ok(Primitive::try_from(input.unbind().bind(gc.into_nogc())).unwrap())
    }
}

pub(crate) fn to_primitive_object<'a, 'gc>(
    agent: &mut Agent,
    input: impl IntoObject<'a>,
    preferred_type: Option<PreferredType>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Primitive<'gc>> {
    let input = input.into_object().bind(gc.nogc());
    // a. Let exoticToPrim be ? GetMethod(input, @@toPrimitive).
    let scoped_input = input.scope(agent, gc.nogc());
    let exotic_to_prim = get_method(
        agent,
        input.into_value().unbind(),
        PropertyKey::Symbol(WellKnownSymbolIndexes::ToPrimitive.into()),
        gc.reborrow(),
    )?;
    // b. If exoticToPrim is not undefined, then
    if let Some(exotic_to_prim) = exotic_to_prim {
        let hint = match preferred_type {
            // i. If preferredType is not present, then
            // 1. Let hint be "default".
            None => BUILTIN_STRING_MEMORY.default,
            // ii. Else if preferredType is STRING, then
            // 1. Let hint be "string".
            Some(PreferredType::String) => BUILTIN_STRING_MEMORY.string,
            // iii. Else,
            // 1. Assert: preferredType is NUMBER.
            // 2. Let hint be "number".
            Some(PreferredType::Number) => BUILTIN_STRING_MEMORY.number,
        };
        // iv. Let result be ? Call(exoticToPrim, input, « hint »).
        let result = call_function(
            agent,
            exotic_to_prim.unbind(),
            scoped_input.get(agent).into_value().unbind(),
            Some(ArgumentsList(&[hint.into()])),
            gc.reborrow(),
        )?
        .unbind();
        let gc = gc.into_nogc();
        let result = result.bind(gc);
        // v. If result is not an Object, return result.
        Primitive::try_from(result).map_err(|_| {
            // vi. Throw a TypeError exception.
            agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid toPrimitive return value",
                gc,
            )
        })
    } else {
        // c. If preferredType is not present, let preferredType be NUMBER.
        // d. Return ? OrdinaryToPrimitive(input, preferredType).
        ordinary_to_primitive(
            agent,
            scoped_input.get(agent),
            preferred_type.unwrap_or(PreferredType::Number),
            gc,
        )
    }
}

/// ### [7.1.1.1 OrdinaryToPrimitive ( O, hint )](https://tc39.es/ecma262/#sec-ordinarytoprimitive)
///
/// The abstract operation OrdinaryToPrimitive takes arguments O (an Object)
/// and hint (STRING or NUMBER) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion.
pub(crate) fn ordinary_to_primitive<'gc>(
    agent: &mut Agent,
    o: Object,
    hint: PreferredType,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Primitive<'gc>> {
    let mut o = o.bind(gc.nogc());
    let to_string_key = PropertyKey::from(BUILTIN_STRING_MEMORY.toString);
    let value_of_key = PropertyKey::from(BUILTIN_STRING_MEMORY.valueOf);
    let method_names = match hint {
        PreferredType::String => {
            // 1. If hint is STRING, then
            // a. Let methodNames be « "toString", "valueOf" ».
            [to_string_key, value_of_key]
        }
        PreferredType::Number => {
            // 2. Else,
            // a. Let methodNames be « "valueOf", "toString" ».
            [value_of_key, to_string_key]
        }
    };
    // 3. For each element name of methodNames, do
    let scoped_o = o.scope(agent, gc.nogc());
    for name in method_names {
        // a. Let method be ? Get(O, name).
        let method = get(agent, o.unbind(), name, gc.reborrow())?
            .unbind()
            .bind(gc.nogc());
        // b. If IsCallable(method) is true, then
        if let Some(method) = is_callable(method, gc.nogc()) {
            // i. Let result be ? Call(method, O).
            let result: Value = call_function(
                agent,
                method.unbind(),
                scoped_o.get(agent).into_value(),
                None,
                gc.reborrow(),
            )?;
            // ii. If result is not an Object, return result.
            if let Ok(result) = Primitive::try_from(result) {
                return Ok(result.unbind().bind(gc.into_nogc()));
            }
        }
        o = scoped_o.get(agent).bind(gc.nogc());
    }
    // 4. Throw a TypeError exception.
    Err(agent.throw_exception_with_static_message(
        ExceptionType::TypeError,
        "Could not convert to primitive",
        gc.nogc(),
    ))
}

/// ### [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/#sec-toboolean)
pub(crate) fn to_boolean(agent: &Agent, argument: Value) -> bool {
    // 1. If argument is a Boolean, return argument.
    if let Value::Boolean(ret) = argument {
        return ret;
    }

    // 2. If argument is one of undefined, null, +0𝔽, -0𝔽, NaN, 0ℤ, or the empty String, return false.
    // TODO: checks for 0ℤ
    if argument.is_undefined()
        || argument.is_null()
        || argument.is_pos_zero(agent)
        || argument.is_neg_zero(agent)
        || argument.is_nan(agent)
        || argument.is_empty_string()
    {
        return false;
    }

    // 3. NOTE: This step is replaced in section B.3.6.1.

    // 4. Return true.
    true
}

/// ### [7.1.3 ToNumeric ( value )](https://tc39.es/ecma262/#sec-tonumeric)
pub(crate) fn to_numeric<'a, 'gc>(
    agent: &mut Agent,
    value: impl IntoValue<'a>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Numeric<'gc>> {
    // 1. Let primValue be ? ToPrimitive(value, number).
    let prim_value =
        to_primitive(agent, value, Some(PreferredType::Number), gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let prim_value = prim_value.bind(gc);

    to_numeric_primitive(agent, prim_value, gc)
}

pub(crate) fn to_numeric_primitive<'a>(
    agent: &mut Agent,
    prim_value: impl IntoPrimitive<'a>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<Numeric<'a>> {
    let prim_value = prim_value.into_primitive();
    // 2. If primValue is a BigInt, return primValue.
    if let Ok(prim_value) = BigInt::try_from(prim_value) {
        return Ok(prim_value.into_numeric());
    }

    // 3. Return ? ToNumber(primValue).
    to_number_primitive(agent, prim_value, gc).map(|n| n.into_numeric())
}

pub(crate) fn try_to_number<'gc>(
    agent: &mut Agent,
    argument: impl IntoValue,
    gc: NoGcScope<'gc, '_>,
) -> Option<JsResult<Number<'gc>>> {
    let argument = argument.into_value();
    if let Ok(argument) = Primitive::try_from(argument) {
        Some(to_number_primitive(agent, argument, gc))
    } else {
        None
    }
}

/// ### [7.1.4 ToNumber ( argument )](https://tc39.es/ecma262/#sec-tonumber)
pub(crate) fn to_number<'a, 'gc>(
    agent: &mut Agent,
    argument: impl IntoValue<'a>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Number<'gc>> {
    let argument = argument.into_value();
    if let Ok(argument) = Primitive::try_from(argument) {
        to_number_primitive(agent, argument, gc.into_nogc())
    } else {
        // 7. Assert: argument is an Object.
        let argument = Object::try_from(argument).unwrap();
        // 8. Let primValue be ? ToPrimitive(argument, number).
        let prim_value =
            to_primitive_object(agent, argument, Some(PreferredType::Number), gc.reborrow())?
                .unbind();
        let gc = gc.into_nogc();
        let prim_value = prim_value.bind(gc);
        // 9. Assert: primValue is not an Object.
        // 10. Return ? ToNumber(primValue).
        to_number_primitive(agent, prim_value, gc)
    }
}

/// ### [7.1.4 ToNumber ( argument )](https://tc39.es/ecma262/#sec-tonumber)
pub(crate) fn to_number_primitive<'gc>(
    agent: &mut Agent,
    argument: Primitive<'gc>,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<Number<'gc>> {
    match argument {
        // 3. If argument is undefined, return NaN.
        Primitive::Undefined => Ok(Number::nan()),
        // 4. If argument is either null or false, return +0𝔽.
        Primitive::Null | Primitive::Boolean(false) => Ok(Number::from(0)),
        // 5. If argument is true, return 1𝔽.
        Primitive::Boolean(true) => Ok(Number::from(1)),
        // 6. If argument is a String, return StringToNumber(argument).
        Primitive::String(str) => Ok(string_to_number(agent, str.into(), gc)),
        Primitive::SmallString(str) => Ok(string_to_number(agent, str.into(), gc)),
        // 2. If argument is either a Symbol or a BigInt, throw a TypeError exception.
        Primitive::Symbol(_) => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "cannot convert symbol to number",
            gc,
        )),
        // 1. If argument is a Number, return argument.
        Primitive::Number(idx) => Ok(idx.into()),
        Primitive::Integer(idx) => Ok(idx.into()),
        Primitive::SmallF64(idx) => Ok(idx.into()),
        Primitive::BigInt(_) | Primitive::SmallBigInt(_) => Err(agent
            .throw_exception_with_static_message(
                ExceptionType::TypeError,
                "cannot convert bigint to number",
                gc,
            )),
    }
}

/// ### [7.1.4.1.1 StringToNumber ( str )](https://tc39.es/ecma262/#sec-stringtonumber)
///
/// The abstract operation StringToNumber takes argument str (a String) and
/// returns a Number.
///
/// Copied from Boa JS engine. Source https://github.com/boa-dev/boa/blob/183e763c32710e4e3ea83ba762cf815b7a89cd1f/core/string/src/lib.rs#L560
///
/// Copyright (c) 2019 Jason Williams
pub(crate) fn string_to_number<'gc>(
    agent: &mut Agent,
    str: String,
    gc: NoGcScope<'gc, '_>,
) -> Number<'gc> {
    // 1. Let literal be ParseText(str, StringNumericLiteral).
    // 2. If literal is a List of errors, return NaN.
    // 3. Return the StringNumericValue of literal.
    let str = str.as_str(agent).trim_matches(is_trimmable_whitespace);
    match str {
        "+Infinity" | "Infinity" => {
            return Number::pos_inf();
        }
        "-Infinity" => {
            return Number::neg_inf();
        }
        "0" | "+0" | "0.0" | "+0.0" => {
            return Number::pos_zero();
        }
        "-0" | "-0.0" => {
            return Number::neg_zero();
        }
        _ => {}
    }

    let mut s = str.bytes();
    let base = match (s.next(), s.next()) {
        (Some(b'0'), Some(b'b' | b'B')) => Some(2),
        (Some(b'0'), Some(b'o' | b'O')) => Some(8),
        (Some(b'0'), Some(b'x' | b'X')) => Some(16),
        // Make sure that no further variants of "infinity" are parsed.
        (Some(b'i' | b'I'), _) => {
            return Number::nan();
        }
        _ => None,
    };

    // Parse numbers that begin with `0b`, `0o` and `0x`.
    if let Some(base) = base {
        let string = &str[2..];
        if string.is_empty() {
            return Number::nan();
        }

        // Fast path
        if let Ok(value) = u32::from_str_radix(string, base) {
            return value.into();
        }

        // Slow path
        let mut value: f64 = 0.0;
        for c in s {
            if let Some(digit) = char::from(c).to_digit(base) {
                value = value.mul_add(f64::from(base), f64::from(digit));
            } else {
                return Number::nan();
            }
        }
        return Number::from_f64(agent, value, gc);
    }

    if let Ok(result) = fast_float::parse(str) {
        Number::from_f64(agent, result, gc)
    } else {
        Number::nan()
    }
}

/// Newtype over a JavaScript integer. The maximum JavaScript safe integer
/// value is at +/- 2^53, after which the f64 value can still represent various
/// larger integers that i64 cannot. ToIntegerOrInfinity is, however, always
/// followed by safe integer checks, hence it makes sense to use only the i64
/// range.
///
/// If the JavaScript number was infinite, then the appropriate i64 minimum or
/// maximum value is used as a sentinel.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct IntegerOrInfinity(i64);

impl IntegerOrInfinity {
    pub(crate) const NEG_INFINITY: Self = Self(i64::MIN);
    pub(crate) const POS_INFINITY: Self = Self(i64::MAX);

    pub(crate) fn is_finite(self) -> bool {
        self.0 != i64::MIN && self.0 != i64::MAX
    }

    pub(crate) fn is_safe_integer(self) -> bool {
        SmallInteger::MIN_NUMBER <= self.0 && self.0 <= SmallInteger::MAX_NUMBER
    }

    pub(crate) fn is_neg_infinity(self) -> bool {
        self.0 == i64::MIN
    }

    pub(crate) fn is_pos_infinity(self) -> bool {
        self.0 == i64::MAX
    }

    pub(crate) fn is_negative(self) -> bool {
        self.0.is_negative()
    }

    pub(crate) fn is_positive(self) -> bool {
        self.0.is_positive()
    }

    pub(crate) fn into_i64(self) -> i64 {
        self.0
    }
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
pub(crate) fn to_integer_or_infinity(
    agent: &mut Agent,
    argument: Value,
    mut gc: GcScope,
) -> JsResult<IntegerOrInfinity> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(int) = argument {
        let int = IntegerOrInfinity(int.into_i64());
        return Ok(int);
    }
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow())?
        .unbind()
        .bind(gc.nogc());

    Ok(to_integer_or_infinity_number(agent, number, gc.nogc()))
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
///
/// This implements steps from 2 onwards.
pub(crate) fn to_integer_or_infinity_number(
    agent: &Agent,
    number: Number,
    gc: NoGcScope,
) -> IntegerOrInfinity {
    // Fast path: The value might've been eg. parsed into an integer.
    if let Number::Integer(int) = number {
        let int = IntegerOrInfinity(int.into_i64());
        return int;
    }

    let number = number.bind(gc);

    // 2. If number is one of NaN, +0𝔽, or -0𝔽, return 0.
    if number.is_nan(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return IntegerOrInfinity(0);
    }

    // 3. If number is +∞𝔽, return +∞.
    if number.is_pos_infinity(agent) {
        return IntegerOrInfinity::POS_INFINITY;
    }

    // 4. If number is -∞𝔽, return -∞.
    if number.is_neg_infinity(agent) {
        return IntegerOrInfinity::NEG_INFINITY;
    }

    // 5. Return truncate(ℝ(number)).
    let number = number.into_f64(agent).trunc() as i64;
    // Note: Make sure converting the f64 didn't take us to our sentinel
    // values.
    let number = if number == i64::MAX {
        i64::MAX - 1
    } else if number == i64::MIN {
        i64::MIN + 1
    } else {
        number
    };
    IntegerOrInfinity(number)
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
//
// This version of the abstract operation attempts to convert the argument
// Value into an integer or infinity without calling any JavaScript code. If
// that cannot be done, `None` is returned. Note that the method can throw an
// error without calling any JavaScript code.
pub(crate) fn try_to_integer_or_infinity(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope,
) -> TryResult<JsResult<IntegerOrInfinity>> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(int) = argument {
        let int = IntegerOrInfinity(int.into_i64());
        return TryResult::Continue(Ok(int));
    }
    // 1. Let number be ? ToNumber(argument).
    let Ok(argument) = Primitive::try_from(argument) else {
        // Converting to Number would require calling into JavaScript code.
        return TryResult::Break(());
    };
    let number = match to_number_primitive(agent, argument, gc) {
        Ok(number) => number,
        Err(err) => {
            return TryResult::Continue(Err(err));
        }
    };

    TryResult::Continue(Ok(to_integer_or_infinity_number(agent, number, gc)))
}

/// ### [7.1.6 ToInt32 ( argument )](https://tc39.es/ecma262/#sec-toint32)
pub(crate) fn to_int32(agent: &mut Agent, argument: Value, mut gc: GcScope) -> JsResult<i32> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly int32 already.
        let int = int.into_i64();
        return Ok(int as i32);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    Ok(to_int32_number(agent, number))
}

/// ### [7.1.6 ToInt32 ( argument )](https://tc39.es/ecma262/#sec-toint32)
///
/// Implements steps 2 to 5 of the abstract operation, callable only with Numbers.
pub(crate) fn to_int32_number(agent: &mut Agent, number: Number) -> i32 {
    if let Number::Integer(int) = number {
        let int = int.into_i64();
        return int as i32;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int32bit be int modulo 2^32.
    // 5. If int32bit ≥ 2^31, return 𝔽(int32bit - 2^32); otherwise return 𝔽(int32bit).
    number.into_f64(agent).trunc() as i64 as i32
}

/// ### [7.1.7 ToUint32 ( argument )](https://tc39.es/ecma262/#sec-touint32)
pub(crate) fn to_uint32(agent: &mut Agent, argument: Value, mut gc: GcScope) -> JsResult<u32> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return Ok(int as u32);
    }
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    Ok(to_uint32_number(agent, number))
}

/// ### [7.1.7 ToUint32 ( argument )](https://tc39.es/ecma262/#sec-touint32)
///
/// Implements steps 2 to 5 of the abstract operation, callable only with Numbers.
pub(crate) fn to_uint32_number(agent: &mut Agent, number: Number) -> u32 {
    if let Number::Integer(int) = number {
        let int = int.into_i64();
        return int as u32;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int32bit be int modulo 2^32.
    // 5. Return 𝔽(int32bit).
    number.into_f64(agent).trunc() as i64 as u32
}

/// ### [7.1.8 ToInt16 ( argument )](https://tc39.es/ecma262/#sec-toint16)
pub(crate) fn to_int16(agent: &mut Agent, argument: Value, mut gc: GcScope) -> JsResult<i16> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly int16 already.
        let int = int.into_i64();
        return Ok(int as i16);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    Ok(to_int16_number(agent, number))
}

pub(crate) fn to_int16_number(agent: &mut Agent, number: Number) -> i16 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly int16 already.
        let int = int.into_i64();
        return int as i16;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int16bit be int modulo 2^16.
    // 5. If int16bit ≥ 2^15, return 𝔽(int16bit - 2^16); otherwise return 𝔽(int16bit).
    number.into_f64(agent).trunc() as i64 as i16
}

/// ### [7.1.9 ToUint16 ( argument )](https://tc39.es/ecma262/#sec-touint16)
pub(crate) fn to_uint16(agent: &mut Agent, argument: Value, mut gc: GcScope) -> JsResult<u16> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint16 already.
        let int = int.into_i64();
        return Ok(int as u16);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    Ok(to_uint16_number(agent, number))
}

pub(crate) fn to_uint16_number(agent: &mut Agent, number: Number) -> u16 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uin16 already.
        let int = int.into_i64();
        return int as u16;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int16bit be int modulo 2^16.
    // Return 𝔽(int16bit).
    number.into_f64(agent).trunc() as i64 as u16
}

/// ### [7.1.10 ToInt8 ( argument )](https://tc39.es/ecma262/#sec-toint8)
pub(crate) fn to_int8(agent: &mut Agent, argument: Value, mut gc: GcScope) -> JsResult<i8> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly int8 already.
        let int = int.into_i64();
        return Ok(int as i8);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    Ok(to_int8_number(agent, number))
}

pub(crate) fn to_int8_number(agent: &mut Agent, number: Number) -> i8 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return int as i8;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int8bit be int modulo 2^8.
    // 5. If int8bit ≥ 2^7, return 𝔽(int8bit - 2^8); otherwise return 𝔽(int8bit).
    number.into_f64(agent).trunc() as i64 as i8
}

/// ### [7.1.11 ToUint8 ( argument )](https://tc39.es/ecma262/#sec-touint8)
pub(crate) fn to_uint8(agent: &mut Agent, argument: Value, mut gc: GcScope) -> JsResult<u8> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return Ok(int as u8);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    Ok(to_uint8_number(agent, number))
}

pub(crate) fn to_uint8_number(agent: &mut Agent, number: Number) -> u8 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return int as u8;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int8bit be int modulo 2^8.
    // 5. Return 𝔽(int8bit).
    number.into_f64(agent).trunc() as i64 as u8
}

/// ### [7.1.12 ToUint8Clamp ( argument )](https://tc39.es/ecma262/#sec-touint8clamp)
pub(crate) fn to_uint8_clamp(agent: &mut Agent, argument: Value, gc: GcScope) -> JsResult<u8> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint8 already.
        let int = int.into_i64().clamp(0, 255);
        return Ok(int as u8);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_uint8_clamp_number(agent, number))
}

pub(crate) fn to_uint8_clamp_number(agent: &mut Agent, number: Number) -> u8 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uint8 already.
        return int.into_i64().clamp(0, 255) as u8;
    }

    // 2. If number is NaN, return +0𝔽.
    if number.is_nan(agent) {
        return 0;
    }

    // 3. Let mv be the extended mathematical value of number.
    let mv = number.into_f64(agent);

    // 4. Let clamped be the result of clamping mv between 0 and 255.
    // 5. Let f be floor(clamped).
    // 6. If clamped < f + 0.5, return 𝔽(f).
    // 7. If clamped > f + 0.5, return 𝔽(f + 1).
    // 8. If f is even, return 𝔽(f). Otherwise, return 𝔽(f + 1).
    mv.clamp(0.0, 255.0).round_ties_even() as u8
}

/// ### [7.1.13 ToBigInt ( argument )](https://tc39.es/ecma262/#sec-tobigint)
#[inline(always)]
pub(crate) fn to_big_int<'a>(
    agent: &mut Agent,
    argument: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<BigInt<'a>> {
    // 1. Let prim be ? ToPrimitive(argument, number).
    let prim = to_primitive(agent, argument, Some(PreferredType::Number), gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let prim = prim.bind(gc);
    to_big_int_primitive(agent, prim, gc)
}

/// ### [7.1.13 ToBigInt ( argument )](https://tc39.es/ecma262/#sec-tobigint)
#[inline(always)]
pub(crate) fn to_big_int_primitive<'a>(
    agent: &mut Agent,
    prim: Primitive,
    gc: NoGcScope<'a, '_>,
) -> JsResult<BigInt<'a>> {
    // 2. Return the value that prim corresponds to in Table 12.
    match prim {
        Primitive::Undefined => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Invalid primitive 'undefined'",
            gc,
        )),
        Primitive::Null => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Invalid primitive 'null'",
            gc,
        )),
        Primitive::Boolean(bool) => {
            if bool {
                Ok(BigInt::from(1))
            } else {
                Ok(BigInt::from(0))
            }
        }
        Primitive::String(idx) => string_to_big_int(agent, idx.into(), gc),
        Primitive::SmallString(data) => string_to_big_int(agent, data.into(), gc),
        Primitive::Symbol(_) => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Cannot convert Symbol to BigInt",
            gc,
        )),
        Primitive::Number(_) | Primitive::Integer(_) | Primitive::SmallF64(_) => Err(agent
            .throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot convert Number to BigInt",
                gc,
            )),
        Primitive::BigInt(idx) => Ok(BigInt::BigInt(idx).bind(gc)),
        Primitive::SmallBigInt(data) => Ok(data.into()),
    }
}

/// ### [7.1.14 StringToBigInt ( str )](https://tc39.es/ecma262/#sec-stringtobigint)
pub(crate) fn string_to_big_int<'a>(
    agent: &mut Agent,
    argument: String,
    nogc: NoGcScope<'a, '_>,
) -> JsResult<BigInt<'a>> {
    // 1. Let text be StringToCodePoints(str).
    // 2. Let literal be ParseText(text, StringIntegerLiteral).
    // 3. If literal is a List of errors, return undefined.

    // Parsing for StringIntegerLiteral (https://tc39.es/ecma262/#prod-StringIntegerLiteral)
    // StringIntegerLiteral is either whitespace only or a StrIntegerLiteral surrounded by
    // optional whitespace.

    let literal = argument.as_str(agent); // Extra line literally just for displaying error

    // 4. Let mv be the MV of literal.
    // 5. Assert: mv is an integer.

    // Rules for MV (https://tc39.es/ecma262/#sec-runtime-semantics-mv-for-stringintegerliteral)
    // Trim whitespace to get rid of optional whitespace.
    let mv = literal.trim_matches(is_trimmable_whitespace);

    // If mv is empty result is Zero
    if mv.is_empty() {
        return Ok(BigInt::from(0));
    }

    // MV should now be StrIntegerLiteral
    // I.e. Either a SignedInteger or a (Binary/Octal/Hex)IntegerLiteral

    // Check for non decimal integer
    let mut s = mv.bytes();
    let base = match (s.next(), s.next()) {
        (Some(b'0'), Some(b'b' | b'B')) => Some(2),
        (Some(b'0'), Some(b'o' | b'O')) => Some(8),
        (Some(b'0'), Some(b'x' | b'X')) => Some(16),
        _ => None,
    };

    // Left with digits only in a particular base
    let string_to_convert = if base.is_some() { &mv[2..] } else { mv };

    // 6. Return ℤ(mv).
    // Parse with the required radix calculated from the base
    let num_big_int =
        num_bigint::BigInt::parse_bytes(string_to_convert.as_bytes(), base.unwrap_or(10));

    if let Some(num_big_int) = num_big_int {
        Ok(BigInt::from_num_bigint(agent, num_big_int))
    } else {
        let message = String::from_string(
            agent,
            format!("Cannot convert {} to a BigInt", literal),
            nogc,
        );
        Err(agent.throw_exception_with_message(ExceptionType::SyntaxError, message))
    }
}

/// ### [7.1.15 ToBigInt64 ( argument )](https://tc39.es/ecma262/#sec-tobigint64)
///
/// The abstract operation ToBigInt64 takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing a BigInt
/// or a throw completion. It converts argument to one of 2**64 BigInt values
/// in the inclusive interval from ℤ(-2**63) to ℤ(2**63 - 1).
pub(crate) fn to_big_int64(agent: &mut Agent, argument: Value, gc: GcScope) -> JsResult<i64> {
    // 1. Let n be ? ToBigInt(argument).
    let n = to_big_int(agent, argument, gc)?;

    Ok(to_big_int64_big_int(agent, n))
}

pub(crate) fn to_big_int64_big_int(agent: &mut Agent, n: BigInt) -> i64 {
    // 2. Let int64bit be ℝ(n) modulo 2**64.
    match n {
        BigInt::BigInt(heap_big_int) => {
            // 3. If int64bit ≥ 2**63, return ℤ(int64bit - 2**64); otherwise return ℤ(int64bit).
            let big_int = &agent[heap_big_int].data;
            let int64bit = big_int.iter_u64_digits().next().unwrap_or(0);
            let int64bit = if big_int.sign() == Sign::Minus {
                u64::MAX - int64bit + 1
            } else {
                int64bit
            };
            i64::from_ne_bytes(int64bit.to_ne_bytes())
        }
        BigInt::SmallBigInt(small_big_int) => small_big_int.into_i64(),
    }
}

/// ### [7.1.16 ToBigUint64 ( argument )](https://tc39.es/ecma262/#sec-tobiguint64)
///
/// The abstract operation ToBigUint64 takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing a BigInt
/// or a throw completion. It converts argument to one of 2**64 BigInt values
/// in the inclusive interval from 0ℤ to ℤ(2**64 - 1).
pub(crate) fn to_big_uint64(agent: &mut Agent, argument: Value, gc: GcScope) -> JsResult<u64> {
    // 1. Let n be ? ToBigInt(argument).
    let n = to_big_int(agent, argument, gc)?;
    Ok(to_big_uint64_big_int(agent, n))
}

pub(crate) fn to_big_uint64_big_int(agent: &mut Agent, n: BigInt) -> u64 {
    // 2. Let int64bit be ℝ(n) modulo 2**64.
    match n {
        BigInt::BigInt(heap_big_int) => {
            // https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=7d82adfe85f7d0ed44ab37a7b2cdf092
            let big_int = &agent[heap_big_int].data;
            let int64bit = big_int.iter_u64_digits().next().unwrap_or(0);
            if big_int.sign() == Sign::Minus {
                u64::MAX - int64bit + 1
            } else {
                int64bit
            }
        }
        BigInt::SmallBigInt(small_big_int) => {
            let int64bit = small_big_int.into_i64();
            int64bit as u64
        }
    }
}

pub(crate) fn try_to_string<'gc>(
    agent: &mut Agent,
    argument: impl IntoValue,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<JsResult<String<'gc>>> {
    let argument = argument.into_value();
    if let Ok(argument) = Primitive::try_from(argument) {
        TryResult::Continue(to_string_primitive(agent, argument, gc))
    } else {
        TryResult::Break(())
    }
}

/// ### [7.1.17 ToString ( argument )](https://tc39.es/ecma262/#sec-tostring)
pub(crate) fn to_string<'a, 'gc>(
    agent: &mut Agent,
    argument: impl IntoValue<'a>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<String<'gc>> {
    let argument = argument.into_value();
    // 1. If argument is a String, return argument.
    if let Ok(argument) = Primitive::try_from(argument) {
        to_string_primitive(agent, argument, gc.into_nogc())
    } else {
        // 9. Assert: argument is an Object.
        assert!(Object::try_from(argument).is_ok());
        // 10. Let primValue be ? ToPrimitive(argument, string).
        let prim_value =
            to_primitive(agent, argument, Some(PreferredType::String), gc.reborrow())?.unbind();
        let gc = gc.into_nogc();
        let prim_value = prim_value.bind(gc);
        // 11. Assert: primValue is not an Object.
        // 12. Return ? ToString(primValue).
        to_string_primitive(agent, prim_value, gc)
    }
}

/// ### [7.1.17 ToString ( argument )](https://tc39.es/ecma262/#sec-tostring)
pub(crate) fn to_string_primitive<'gc>(
    agent: &mut Agent,
    argument: Primitive<'gc>,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<String<'gc>> {
    // 1. If argument is a String, return argument.
    match argument {
        // 3. If argument is undefined, return "undefined".
        Primitive::Undefined => Ok(BUILTIN_STRING_MEMORY.undefined),
        // 4. If argument is null, return "null".
        Primitive::Null => Ok(BUILTIN_STRING_MEMORY.null),
        Primitive::Boolean(value) => {
            if value {
                // 5. If argument is true, return "true".
                Ok(BUILTIN_STRING_MEMORY.r#true)
            } else {
                // 6. If argument is false, return "false".
                Ok(BUILTIN_STRING_MEMORY.r#false)
            }
        }
        Primitive::String(idx) => Ok(String::String(idx)),
        Primitive::SmallString(data) => Ok(String::SmallString(data)),
        // 2. If argument is a Symbol, throw a TypeError exception.
        Primitive::Symbol(_) => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Cannot turn Symbol into string",
            gc,
        )),
        // 7. If argument is a Number, return Number::toString(argument, 10).
        Primitive::Number(_) | Primitive::Integer(_) | Primitive::SmallF64(_) => {
            Ok(Number::to_string_radix_10(agent, Number::try_from(argument).unwrap(), gc).unbind())
        }
        // 8. If argument is a BigInt, return BigInt::toString(argument, 10).
        Primitive::BigInt(_) | Primitive::SmallBigInt(_) => {
            Ok(BigInt::to_string_radix_10(agent, BigInt::try_from(argument).unwrap(), gc).unbind())
        }
    }
}

/// ### [7.1.18 ToObject ( argument )](https://tc39.es/ecma262/#sec-toobject)
///
/// The abstract operation ToObject takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing an Object
/// or a throw completion. It converts argument to a value of type Object
/// according to [Table 13](https://tc39.es/ecma262/#table-toobject-conversions):
pub(crate) fn to_object<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<Object<'a>> {
    match argument {
        Value::Undefined | Value::Null => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Argument cannot be converted into an object",
            gc,
        )),
        // Return a new Boolean object whose [[BooleanData]] internal slot is set to argument.
        Value::Boolean(bool) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::Boolean(bool),
            })
            .into_object()),
        // Return a new String object whose [[StringData]] internal slot is set to argument.
        Value::String(str) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::String(str),
            })
            .into_object()),
        Value::SmallString(str) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::SmallString(str),
            })
            .into_object()),
        // Return a new Symbol object whose [[SymbolnData]] internal slot is set to argument.
        Value::Symbol(symbol) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::Symbol(symbol),
            })
            .into_object()),
        // Return a new Number object whose [[NumberData]] internal slot is set to argument.
        Value::Number(number) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::Number(number),
            })
            .into_object()),
        Value::Integer(integer) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::Integer(integer),
            })
            .into_object()),
        Value::SmallF64(float) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::Float(float),
            })
            .into_object()),
        // Return a new BigInt object whose [[BigIntData]] internal slot is set to argument.
        Value::BigInt(bigint) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::BigInt(bigint),
            })
            .into_object()),
        Value::SmallBigInt(bigint) => Ok(agent
            .heap
            .create(PrimitiveObjectHeapData {
                object_index: None,
                data: PrimitiveObjectData::SmallBigInt(bigint),
            })
            .into_object()),
        _ => Ok(Object::try_from(argument).unwrap()),
    }
}

/// ### [7.1.19 ToPropertyKey ( argument )](https://tc39.es/ecma262/#sec-topropertykey)
pub(crate) fn to_property_key<'a>(
    agent: &mut Agent,
    argument: impl IntoValue,
    gc: GcScope<'a, '_>,
) -> JsResult<PropertyKey<'a>> {
    // Note: Fast path and non-standard special case combined. Usually the
    // argument is already a valid property key. We also need to parse integer
    // strings back into integer property keys.
    if let TryResult::Continue(simple_result) = to_property_key_simple(agent, argument, gc.nogc()) {
        return Ok(simple_result.unbind().bind(gc.into_nogc()));
    }

    // If the argument is not a simple property key, it means that we may need
    // to call JavaScript or at least allocate a string on the heap.
    // We call ToPrimitive in case we're dealing with an object.
    to_property_key_complex(agent, argument, gc)
}

/// ### [7.1.19 ToPropertyKey ( argument )](https://tc39.es/ecma262/#sec-topropertykey)
///
/// This method handles Nova's special case of PropertyKey containing all safe
/// integers as PropertyKey::Integer values. Additionally, this only handles
/// cases that do not require calling any JavaScript code or allocating on the
/// heap. The cases that are handled are:
///
/// - Safe integer numbers
/// - Safe integet bigints
/// - Negative zero
/// - Stringified safe integers
/// - Strings
/// - Symbols
/// - undefined
/// - null
/// - true
/// - false
///
/// If a complex case is found, the function returns None to indicate that the
/// caller should handle the uncommon case.
pub(crate) fn to_property_key_simple<'a>(
    agent: &Agent,
    argument: impl IntoValue,
    _: NoGcScope<'a, '_>,
) -> TryResult<PropertyKey<'a>> {
    let argument = argument.into_value();
    match argument {
        Value::String(_) | Value::SmallString(_) => {
            let (str, string_key) = match &argument {
                Value::String(x) => (agent[*x].as_str(), PropertyKey::String(*x)),
                Value::SmallString(x) => (x.as_str(), PropertyKey::SmallString(*x)),
                _ => unreachable!(),
            };
            if let Some(key) = parse_string_to_integer_property_key(str) {
                TryResult::Continue(key)
            } else {
                TryResult::Continue(string_key)
            }
        }
        Value::Integer(x) => TryResult::Continue(PropertyKey::Integer(x)),
        Value::SmallF64(x) if x.into_f64() == -0.0 => {
            TryResult::Continue(PropertyKey::Integer(0.into()))
        }
        Value::Symbol(x) => TryResult::Continue(PropertyKey::Symbol(x)),
        Value::SmallBigInt(x)
            if (SmallInteger::MIN_NUMBER..=SmallInteger::MAX_NUMBER).contains(&x.into_i64()) =>
        {
            TryResult::Continue(PropertyKey::Integer(x.into_inner()))
        }
        Value::Undefined => TryResult::Continue(PropertyKey::from(BUILTIN_STRING_MEMORY.undefined)),
        Value::Null => TryResult::Continue(PropertyKey::from(BUILTIN_STRING_MEMORY.null)),
        Value::Boolean(bool) => {
            if bool {
                TryResult::Continue(PropertyKey::from(BUILTIN_STRING_MEMORY.r#true))
            } else {
                TryResult::Continue(PropertyKey::from(BUILTIN_STRING_MEMORY.r#false))
            }
        }
        _ => TryResult::Break(()),
    }
}

pub(crate) fn to_property_key_complex<'a>(
    agent: &mut Agent,
    argument: impl IntoValue,
    mut gc: GcScope<'a, '_>,
) -> JsResult<PropertyKey<'a>> {
    // 1. Let key be ? ToPrimitive(argument, hint String).
    let key = to_primitive(agent, argument, Some(PreferredType::String), gc.reborrow())?.unbind();
    let gc = gc.into_nogc();
    let key = key.bind(gc);

    // 2. If Type(key) is Symbol, then
    //    a. Return key.
    // Note: We can reuse the fast path and non-standard special case handler:
    // If the property key was an object, it is now a primitive. We need to do
    // our non-standard parsing of integer strings back into integer property
    // keys here as well.
    if let TryResult::Continue(key) = to_property_key_simple(agent, key, gc) {
        Ok(key)
    } else {
        // Key was still not simple: This mean it's a heap allocated f64,
        // BigInt, or non-negative-zero f32: These should never be safe
        // integers and thus will never be PropertyKey::Integer after
        // stringifying.

        // 3. Return ! ToString(key).
        Ok(to_string_primitive(agent, key, gc).unwrap().into())
    }
}

pub(crate) fn parse_string_to_integer_property_key(str: &str) -> Option<PropertyKey<'static>> {
    // i64::from_string will accept eg. 0123 as 123 but JS property keys do
    // not agree. Hence, only "0" can start with "0", all other integer
    // keys must start with one of "1".."9".
    if str == "0" {
        return Some(0.into());
    } else if !str.is_empty()
        && (str.starts_with('-') || (b'1'..=b'9').contains(&str.as_bytes()[0]))
    {
        if let Ok(result) = str.parse::<i64>() {
            if (SmallInteger::MIN_NUMBER..=SmallInteger::MAX_NUMBER).contains(&result) {
                return Some(SmallInteger::try_from(result).unwrap().into());
            }
        }
    }
    None
}

/// ### [7.1.20 ToLength ( argument )](https://tc39.es/ecma262/#sec-tolength)
pub(crate) fn to_length(agent: &mut Agent, argument: Value, gc: GcScope) -> JsResult<i64> {
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let len be ? ToIntegerOrInfinity(argument).
    let len = to_integer_or_infinity(agent, argument, gc)?.into_i64();

    // 2. If len ≤ 0, return +0𝔽.
    if len <= 0 {
        return Ok(0);
    }

    // 3. Return 𝔽(min(len, 2**53 - 1)).
    Ok(len.min(SmallInteger::MAX_NUMBER))
}

/// ### [7.1.20 ToLength ( argument )](https://tc39.es/ecma262/#sec-tolength)
pub(crate) fn try_to_length(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope,
) -> TryResult<JsResult<i64>> {
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let len be ? ToIntegerOrInfinity(argument).
    let len = match try_to_integer_or_infinity(agent, argument, gc)? {
        Ok(len) => len.into_i64(),
        Err(err) => return TryResult::Continue(Err(err)),
    };

    // 2. If len ≤ 0, return +0𝔽.
    if len <= 0 {
        return TryResult::Continue(Ok(0));
    }

    // 3. Return 𝔽(min(len, 2**53 - 1)).
    TryResult::Continue(Ok(len.min(SmallInteger::MAX_NUMBER)))
}

/// ### [7.1.21 CanonicalNumericIndexString ( argument )](https://tc39.es/ecma262/#sec-canonicalnumericindexstring)
pub(crate) fn canonical_numeric_index_string<'gc>(
    agent: &mut Agent,
    argument: String<'gc>,
    gc: NoGcScope<'gc, '_>,
) -> Option<Number<'gc>> {
    // 1. If argument is "-0", return -0𝔽.
    if argument == BUILTIN_STRING_MEMORY.__0 {
        return Some(Number::neg_zero());
    }

    // 2. Let n be ! ToNumber(argument).
    let n = to_number_primitive(agent, argument.into_primitive(), gc).unwrap();

    // 3. If ! ToString(n) is argument, return n.
    if to_string_primitive(agent, n.into_primitive(), gc).unwrap() == argument {
        return Some(n);
    }

    // 4. Return undefined.
    None
}

/// ### [7.1.22 ToIndex ( value )](https://tc39.es/ecma262/#sec-toindex)
pub(crate) fn to_index(agent: &mut Agent, argument: Value, mut gc: GcScope) -> JsResult<i64> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(integer) = argument {
        let integer = integer.into_i64();
        if !(0..=(SmallInteger::MAX_NUMBER)).contains(&integer) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Index is out of range",
                gc.nogc(),
            ));
        }
        return Ok(integer);
    }
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let integer be ? ToIntegerOrInfinity(value).
    let integer = to_integer_or_infinity(agent, argument, gc.reborrow())?.into_i64();

    // 2. If integer is not in the inclusive interval from 0 to 2**53 - 1, throw a RangeError exception.
    if !(0..=(SmallInteger::MAX_NUMBER)).contains(&integer) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Index is out of range",
            gc.nogc(),
        ));
    }

    // 3. Return integer.
    Ok(integer)
}

/// ### [7.1.22 ToIndex ( value )](https://tc39.es/ecma262/#sec-toindex)
pub(crate) fn try_to_index(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope,
) -> TryResult<JsResult<i64>> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(integer) = argument {
        let integer = integer.into_i64();
        if !(0..=(SmallInteger::MAX_NUMBER)).contains(&integer) {
            return TryResult::Continue(Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Index is out of range",
                gc,
            )));
        }
        return TryResult::Continue(Ok(integer));
    }
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let integer be ? ToIntegerOrInfinity(value).
    let integer = match try_to_integer_or_infinity(agent, argument, gc)? {
        Ok(i) => i.into_i64(),
        Err(err) => return TryResult::Continue(Err(err)),
    };

    // 2. If integer is not in the inclusive interval from 0 to 2**53 - 1, throw a RangeError exception.
    if !(0..=(SmallInteger::MAX_NUMBER)).contains(&integer) {
        return TryResult::Continue(Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Index is out of range",
            gc,
        )));
    }

    // 3. Return integer.
    TryResult::Continue(Ok(integer))
}

/// Helper function to check if a `char` is trimmable.
///
/// Copied from Boa JS engine. Source https://github.com/boa-dev/boa/blob/183e763c32710e4e3ea83ba762cf815b7a89cd1f/core/string/src/lib.rs#L51
///
/// Copyright (c) 2019 Jason Williams
pub const fn is_trimmable_whitespace(c: char) -> bool {
    // The rust implementation of `trim` does not regard the same characters whitespace as ecma standard does
    //
    // Rust uses \p{White_Space} by default, which also includes:
    // `\u{0085}' (next line)
    // And does not include:
    // '\u{FEFF}' (zero width non-breaking space)
    // Explicit whitespace: https://tc39.es/ecma262/#sec-white-space
    matches!(
        c,
        '\u{0009}' | '\u{000B}' | '\u{000C}' | '\u{0020}' | '\u{00A0}' | '\u{FEFF}' |
    // Unicode Space_Separator category
    '\u{1680}' | '\u{2000}'
            ..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}' |
    // Line terminators: https://tc39.es/ecma262/#sec-line-terminators
    '\u{000A}' | '\u{000D}' | '\u{2028}' | '\u{2029}'
    )
}
