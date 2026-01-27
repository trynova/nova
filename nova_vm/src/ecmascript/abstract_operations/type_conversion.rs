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

use std::convert::Infallible;

use num_bigint::Sign;
use wtf8::Wtf8;

use crate::{
    ecmascript::{
    SmallInteger,
        Agent, ArgumentsList, ExceptionType, JsResult, PrimitiveObjectData, PrimitiveObjectRecord,
        TryError, TryResult, js_result_into_try,
        types::{
            BUILTIN_STRING_MEMORY, BigInt, Number, Numeric, Object, Primitive, PropertyKey, String,
            Value,
        },
    },
    engine::{
        Bindable, GcScope, NoGcScope, trivially_bindable,
        Scopable,
    },
    heap::{ArenaAccess, CreateHeapData, WellKnownSymbolIndexes},
};

use super::{
    operations_on_objects::{call_function, get, get_method},
    testing_and_comparison::{is_callable, require_object_coercible},
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum PreferredType {
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
/// may use the optional hint preferredType to favour that type.
///
/// > NOTE: When ToPrimitive is called without a hint, then it generally
/// > behaves as if the hint were NUMBER. However, objects may over-ride this
/// > behaviour by defining a @@toPrimitive method. Of the objects defined in
/// > this specification only Dates (see 21.4.4.45) and Symbol objects (see
/// > 20.4.3.5) over-ride the default ToPrimitive behaviour. Dates treat the
/// > absence of a hint as if the hint were STRING.
pub(crate) fn to_primitive<'a, 'gc>(
    agent: &mut Agent,
    input: impl Into<Value<'a>>,
    preferred_type: Option<PreferredType>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Primitive<'gc>> {
    let input = input.into().bind(gc.nogc());
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
    input: impl Into<Object<'a>>,
    preferred_type: Option<PreferredType>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Primitive<'gc>> {
    let input = input.into().bind(gc.nogc());
    // a. Let exoticToPrim be ? GetMethod(input, @@toPrimitive).
    let scoped_input = input.scope(agent, gc.nogc());
    let exotic_to_prim = get_method(
        agent,
        input.unbind().into(),
        PropertyKey::Symbol(WellKnownSymbolIndexes::ToPrimitive.into()),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
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
            scoped_input.get(agent).unbind().into(),
            Some(ArgumentsList::from_mut_slice(&mut [hint.into()])),
            gc.reborrow(),
        )
        .unbind()?;
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
) -> JsResult<'gc, Primitive<'gc>> {
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
        let method = get(agent, o.unbind(), name, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // b. If IsCallable(method) is true, then
        if let Some(method) = is_callable(method, gc.nogc()) {
            // i. Let result be ? Call(method, O).
            let result: Value = call_function(
                agent,
                method.unbind(),
                scoped_o.get(agent).into(),
                None,
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
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
        gc.into_nogc(),
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
    value: impl Into<Value<'a>>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Numeric<'gc>> {
    // 1. Let primValue be ? ToPrimitive(value, number).
    let prim_value =
        to_primitive(agent, value, Some(PreferredType::Number), gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let prim_value = prim_value.bind(gc);

    to_numeric_primitive(agent, prim_value, gc)
}

pub(crate) fn to_numeric_primitive<'a>(
    agent: &mut Agent,
    prim_value: impl Into<Primitive<'a>>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Numeric<'a>> {
    let prim_value = prim_value.into();
    // 2. If primValue is a BigInt, return primValue.
    if let Ok(prim_value) = BigInt::try_from(prim_value) {
        return Ok(prim_value.into());
    }

    // 3. Return ? ToNumber(primValue).
    to_number_primitive(agent, prim_value, gc).map(|n| n.into())
}

/// ### [7.1.4 ToNumber ( argument )](https://tc39.es/ecma262/#sec-tonumber)
pub(crate) fn to_number<'a, 'gc>(
    agent: &mut Agent,
    argument: impl Into<Value<'a>>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Number<'gc>> {
    let argument = argument.into().bind(gc.nogc());
    if let Ok(argument) = Primitive::try_from(argument) {
        to_number_primitive(agent, argument.unbind(), gc.into_nogc())
    } else {
        // 7. Assert: argument is an Object.
        let argument = Object::try_from(argument).unwrap();
        // 8. Let primValue be ? ToPrimitive(argument, number).
        let prim_value = to_primitive_object(
            agent,
            argument.unbind(),
            Some(PreferredType::Number),
            gc.reborrow(),
        )
        .unbind()?;
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
    argument: Primitive,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, Number<'gc>> {
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
        Primitive::Number(idx) => Ok(idx.unbind().bind(gc).into()),
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
    let str = str.to_string_lossy_(agent);
    let str = str.trim_matches(is_trimmable_whitespace);
    match str {
        "+Infinity" | "Infinity" => {
            return Number::pos_inf();
        }
        "-Infinity" => {
            return Number::neg_inf();
        }
        "" | "0" | "+0" | "0.0" | "+0.0" => {
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
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct IntegerOrInfinity(i64);

impl core::fmt::Display for IntegerOrInfinity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_finite() {
            self.0.fmt(f)
        } else if self.is_neg_infinity() {
            f.write_str("-Infinity")
        } else {
            f.write_str("Infinity")
        }
    }
}

impl core::fmt::Debug for IntegerOrInfinity {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as core::fmt::Display>::fmt(self, f)
    }
}

impl IntegerOrInfinity {
    pub(crate) const NEG_INFINITY: Self = Self(i64::MIN);
    pub(crate) const POS_INFINITY: Self = Self(i64::MAX);

    pub(crate) fn is_finite(self) -> bool {
        self.0 != i64::MIN && self.0 != i64::MAX
    }

    pub(crate) fn is_safe_integer(self) -> bool {
        SmallInteger::MIN <= self.0 && self.0 <= SmallInteger::MAX
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

    pub(crate) fn into_i64(self) -> i64 {
        self.0
    }
}

impl PartialEq<i64> for IntegerOrInfinity {
    fn eq(&self, other: &i64) -> bool {
        self.is_safe_integer() && self.0 == *other
    }
}

impl PartialOrd<i64> for IntegerOrInfinity {
    fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> {
        if !self.is_safe_integer() {
            return None;
        }
        self.0.partial_cmp(other)
    }
}

impl PartialEq<IntegerOrInfinity> for i64 {
    fn eq(&self, other: &IntegerOrInfinity) -> bool {
        other.eq(self)
    }
}
impl PartialOrd<IntegerOrInfinity> for i64 {
    fn partial_cmp(&self, other: &IntegerOrInfinity) -> Option<std::cmp::Ordering> {
        if !other.is_safe_integer() {
            return None;
        }
        self.partial_cmp(&other.0)
    }
}

trivially_bindable!(IntegerOrInfinity);

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
#[cfg(feature = "date")]
pub(crate) fn to_integer_or_infinity_f64(number: f64) -> f64 {
    // `ToIntegerOrInfinity ( argument )`
    if number.is_nan() || number == 0.0 {
        // 2. If number is NaN, +0𝔽, or -0𝔽, return 0.
        0.0
    } else if number == f64::INFINITY {
        // 3. If number is +∞𝔽, return +∞.
        f64::INFINITY
    } else if number == f64::NEG_INFINITY {
        // 4. If number is -∞𝔽, return -∞.
        f64::NEG_INFINITY
    } else {
        // 5. Let integer be floor(abs(ℝ(number))).
        // 6. If number < +0𝔽, set integer to -integer.
        // 7. Return integer.
        number.trunc()
    }
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
pub(crate) fn to_integer_or_infinity<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, IntegerOrInfinity> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(int) = argument {
        let int = IntegerOrInfinity(int.into_i64());
        return Ok(int);
    }
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_integer_or_infinity_number(agent, number))
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
///
/// This implements steps from 2 onwards.
pub(crate) fn to_integer_or_infinity_number(agent: &Agent, number: Number) -> IntegerOrInfinity {
    // Fast path: The value might've been eg. parsed into an integer.
    if let Number::Integer(int) = number {
        let int = IntegerOrInfinity(int.into_i64());
        return int;
    }

    // 2. If number is one of NaN, +0𝔽, or -0𝔽, return 0.
    if number.is_nan_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return IntegerOrInfinity(0);
    }

    // 3. If number is +∞𝔽, return +∞.
    if number.is_pos_infinity_(agent) {
        return IntegerOrInfinity::POS_INFINITY;
    }

    // 4. If number is -∞𝔽, return -∞.
    if number.is_neg_infinity_(agent) {
        return IntegerOrInfinity::NEG_INFINITY;
    }

    // 5. Return truncate(ℝ(number)).
    let number = number.into_f64_(agent).trunc() as i64;
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
pub(crate) fn try_to_integer_or_infinity<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope<'a, '_>,
) -> TryResult<'a, IntegerOrInfinity> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(int) = argument {
        let int = IntegerOrInfinity(int.into_i64());
        return TryResult::Continue(int);
    }
    // 1. Let number be ? ToNumber(argument).
    let Ok(argument) = Primitive::try_from(argument) else {
        // Converting to Number would require calling into JavaScript code.
        return TryError::GcError.into();
    };
    let number = js_result_into_try(to_number_primitive(agent, argument, gc))?;

    TryResult::Continue(to_integer_or_infinity_number(agent, number))
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
#[cfg(feature = "atomics")]
pub(crate) fn to_integer_number_or_infinity<'a>(
    agent: &mut Agent,
    argument: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Number<'a>> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(int) = argument {
        return Ok(Number::Integer(int));
    }
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    // Fast path: The value might've been eg. parsed into an integer.
    if let Number::Integer(int) = number {
        return Ok(Number::Integer(int));
    }

    // 2. If number is one of NaN, +0𝔽, or -0𝔽, return 0.
    if number.is_nan_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return Ok(Number::from(0));
    }

    // 3. If number is +∞𝔽, return +∞.
    if number.is_pos_infinity_(agent) {
        return Ok(Number::pos_inf());
    }

    // 4. If number is -∞𝔽, return -∞.
    if number.is_neg_infinity_(agent) {
        return Ok(Number::neg_inf());
    }

    // 5. Return truncate(ℝ(number)).
    Ok(number.unbind().truncate(agent, gc))
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
#[cfg(feature = "atomics")]
pub(crate) fn number_convert_to_integer_or_infinity<'a>(
    agent: &mut Agent,
    number: Number<'a>,
    gc: NoGcScope<'a, '_>,
) -> Number<'a> {
    // Fast path: A safe integer is already an integer.
    if let Number::Integer(int) = number {
        return Number::Integer(int);
    }

    // 2. If number is one of NaN, +0𝔽, or -0𝔽, return 0.
    if number.is_nan_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return Number::from(0);
    }

    // 3. If number is +∞𝔽, return +∞.
    if number.is_pos_infinity_(agent) {
        return Number::pos_inf();
    }

    // 4. If number is -∞𝔽, return -∞.
    if number.is_neg_infinity_(agent) {
        return Number::neg_inf();
    }

    // 5. Return truncate(ℝ(number)).
    number.unbind().truncate(agent, gc)
}

/// ### [7.1.6 ToInt32 ( argument )](https://tc39.es/ecma262/#sec-toint32)
pub(crate) fn to_int32<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, i32> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly int32 already.
        let int = int.into_i64();
        return Ok(int as i32);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_int32_number(agent, number))
}

/// ### [7.1.6 ToInt32 ( argument )](https://tc39.es/ecma262/#sec-toint32)
///
/// Implements steps 2 to 5 of the abstract operation, callable only with Numbers.
pub(crate) fn to_int32_number(agent: &Agent, number: Number) -> i32 {
    if let Number::Integer(int) = number {
        let int = int.into_i64();
        return int as i32;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int32bit be int modulo 2^32.
    // 5. If int32bit ≥ 2^31, return 𝔽(int32bit - 2^32); otherwise return 𝔽(int32bit).
    number.into_f64_(agent).trunc() as i64 as i32
}

/// ### [7.1.7 ToUint32 ( argument )](https://tc39.es/ecma262/#sec-touint32)
pub(crate) fn to_uint32<'a>(
    agent: &mut Agent,
    argument: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, u32> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return Ok(int as u32);
    }
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let number = number.bind(gc);

    Ok(to_uint32_number(agent, number))
}

/// ### [7.1.7 ToUint32 ( argument )](https://tc39.es/ecma262/#sec-touint32)
///
/// Implements steps 2 to 5 of the abstract operation, callable only with Numbers.
pub(crate) fn to_uint32_number(agent: &Agent, number: Number) -> u32 {
    if let Number::Integer(int) = number {
        let int = int.into_i64();
        return int as u32;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int32bit be int modulo 2^32.
    // 5. Return 𝔽(int32bit).
    number.into_f64_(agent).trunc() as i64 as u32
}

/// ### [7.1.8 ToInt16 ( argument )](https://tc39.es/ecma262/#sec-toint16)
pub(crate) fn to_int16<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, i16> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly int16 already.
        let int = int.into_i64();
        return Ok(int as i16);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_int16_number(agent, number))
}

pub(crate) fn to_int16_number(agent: &Agent, number: Number) -> i16 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly int16 already.
        let int = int.into_i64();
        return int as i16;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int16bit be int modulo 2^16.
    // 5. If int16bit ≥ 2^15, return 𝔽(int16bit - 2^16); otherwise return 𝔽(int16bit).
    number.into_f64_(agent).trunc() as i64 as i16
}

/// ### [7.1.9 ToUint16 ( argument )](https://tc39.es/ecma262/#sec-touint16)
pub(crate) fn to_uint16<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, u16> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint16 already.
        let int = int.into_i64();
        return Ok(int as u16);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_uint16_number(agent, number))
}

pub(crate) fn to_uint16_number(agent: &Agent, number: Number) -> u16 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uin16 already.
        let int = int.into_i64();
        return int as u16;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int16bit be int modulo 2^16.
    // Return 𝔽(int16bit).
    number.into_f64_(agent).trunc() as i64 as u16
}

/// ### [7.1.10 ToInt8 ( argument )](https://tc39.es/ecma262/#sec-toint8)
pub(crate) fn to_int8<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, i8> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly int8 already.
        let int = int.into_i64();
        return Ok(int as i8);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_int8_number(agent, number))
}

pub(crate) fn to_int8_number(agent: &Agent, number: Number) -> i8 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return int as i8;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int8bit be int modulo 2^8.
    // 5. If int8bit ≥ 2^7, return 𝔽(int8bit - 2^8); otherwise return 𝔽(int8bit).
    number.into_f64_(agent).trunc() as i64 as i8
}

/// ### [7.1.11 ToUint8 ( argument )](https://tc39.es/ecma262/#sec-touint8)
pub(crate) fn to_uint8<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, u8> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return Ok(int as u8);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_uint8_number(agent, number))
}

pub(crate) fn to_uint8_number(agent: &Agent, number: Number) -> u8 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uint32 already.
        let int = int.into_i64();
        return int as u8;
    }

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite_(agent) || number.is_pos_zero_(agent) || number.is_neg_zero_(agent) {
        return 0;
    }

    // 3. Let int be truncate(ℝ(number)).
    // 4. Let int8bit be int modulo 2^8.
    // 5. Return 𝔽(int8bit).
    number.into_f64_(agent).trunc() as i64 as u8
}

/// ### [7.1.12 ToUint8Clamp ( argument )](https://tc39.es/ecma262/#sec-touint8clamp)
pub(crate) fn to_uint8_clamp<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, u8> {
    if let Value::Integer(int) = argument {
        // Fast path: Integer value is very nearly uint8 already.
        let int = int.into_i64().clamp(0, 255);
        return Ok(int as u8);
    }

    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument, gc)?;

    Ok(to_uint8_clamp_number(agent, number))
}

pub(crate) fn to_uint8_clamp_number(agent: &Agent, number: Number) -> u8 {
    if let Number::Integer(int) = number {
        // Fast path: Integer value is very nearly uint8 already.
        return int.into_i64().clamp(0, 255) as u8;
    }

    // 2. If number is NaN, return +0𝔽.
    if number.is_nan_(agent) {
        return 0;
    }

    // 3. Let mv be the extended mathematical value of number.
    let mv = number.into_f64_(agent);

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
) -> JsResult<'a, BigInt<'a>> {
    // 1. Let prim be ? ToPrimitive(argument, number).
    let prim =
        to_primitive(agent, argument, Some(PreferredType::Number), gc.reborrow()).unbind()?;
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
) -> JsResult<'a, BigInt<'a>> {
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
) -> JsResult<'a, BigInt<'a>> {
    // 1. Let text be StringToCodePoints(str).
    // 2. Let literal be ParseText(text, StringIntegerLiteral).
    // 3. If literal is a List of errors, return undefined.

    // Parsing for StringIntegerLiteral (https://tc39.es/ecma262/#prod-StringIntegerLiteral)
    // StringIntegerLiteral is either whitespace only or a StrIntegerLiteral surrounded by
    // optional whitespace.

    let literal = argument.to_string_lossy_(agent); // Extra line literally just for displaying error

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
        let message =
            String::from_string(agent, format!("Cannot convert {literal} to a BigInt"), nogc);
        Err(agent.throw_exception_with_message(ExceptionType::SyntaxError, message, nogc))
    }
}

/// ### [7.1.15 ToBigInt64 ( argument )](https://tc39.es/ecma262/#sec-tobigint64)
///
/// The abstract operation ToBigInt64 takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing a BigInt
/// or a throw completion. It converts argument to one of 2**64 BigInt values
/// in the inclusive interval from ℤ(-2**63) to ℤ(2**63 - 1).
pub(crate) fn to_big_int64<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, i64> {
    // 1. Let n be ? ToBigInt(argument).
    let n = to_big_int(agent, argument, gc)?;

    Ok(to_big_int64_big_int(agent, n))
}

pub(crate) fn to_big_int64_big_int(agent: &Agent, n: BigInt) -> i64 {
    // 2. Let int64bit be ℝ(n) modulo 2**64.
    match n {
        BigInt::BigInt(heap_big_int) => {
            // 3. If int64bit ≥ 2**63, return ℤ(int64bit - 2**64); otherwise return ℤ(int64bit).
            let big_int = heap_big_int.get(agent);
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
pub(crate) fn to_big_uint64<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, u64> {
    // 1. Let n be ? ToBigInt(argument).
    let n = to_big_int(agent, argument, gc)?;
    Ok(to_big_uint64_big_int(agent, n))
}

pub(crate) fn to_big_uint64_big_int(agent: &Agent, n: BigInt) -> u64 {
    // 2. Let int64bit be ℝ(n) modulo 2**64.
    match n {
        BigInt::BigInt(heap_big_int) => {
            // https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=7d82adfe85f7d0ed44ab37a7b2cdf092
            let big_int = &heap_big_int.get(agent).data;
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

pub(crate) fn try_to_string<'a, 'gc>(
    agent: &mut Agent,
    argument: impl Into<Value<'a>>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, String<'gc>> {
    let argument = argument.into().bind(gc);
    if let Ok(argument) = Primitive::try_from(argument) {
        js_result_into_try(to_string_primitive(agent, argument, gc))
    } else {
        TryError::GcError.into()
    }
}

/// ### [7.1.17 ToString ( argument )](https://tc39.es/ecma262/#sec-tostring)
pub(crate) fn to_string<'a, 'gc>(
    agent: &mut Agent,
    argument: impl Into<Value<'a>>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, String<'gc>> {
    let argument = argument.into().bind(gc.nogc());
    // 1. If argument is a String, return argument.
    if let Ok(argument) = Primitive::try_from(argument) {
        to_string_primitive(agent, argument.unbind(), gc.into_nogc())
    } else {
        // 9. Assert: argument is an Object.
        assert!(Object::try_from(argument).is_ok());
        // 10. Let primValue be ? ToPrimitive(argument, string).
        let prim_value = to_primitive(
            agent,
            argument.unbind(),
            Some(PreferredType::String),
            gc.reborrow(),
        )
        .unbind()?;
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
) -> JsResult<'gc, String<'gc>> {
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
) -> JsResult<'a, Object<'a>> {
    let argument = argument.bind(gc);
    require_object_coercible(agent, argument, gc)?;
    match argument {
        Value::Undefined | Value::Null => unreachable!(),
        // Return a new Boolean object whose [[BooleanData]] internal slot is set to argument.
        Value::Boolean(bool) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::Boolean(bool),
            })
            .into()),
        // Return a new String object whose [[StringData]] internal slot is set to argument.
        Value::String(str) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::String(str.unbind()),
            })
            .into()),
        Value::SmallString(str) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::SmallString(str),
            })
            .into()),
        // Return a new Symbol object whose [[SymbolnData]] internal slot is set to argument.
        Value::Symbol(symbol) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::Symbol(symbol.unbind()),
            })
            .into()),
        // Return a new Number object whose [[NumberData]] internal slot is set to argument.
        Value::Number(number) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::Number(number.unbind()),
            })
            .into()),
        Value::Integer(integer) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::Integer(integer),
            })
            .into()),
        Value::SmallF64(float) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::SmallF64(float),
            })
            .into()),
        // Return a new BigInt object whose [[BigIntData]] internal slot is set to argument.
        Value::BigInt(bigint) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::BigInt(bigint.unbind()),
            })
            .into()),
        Value::SmallBigInt(bigint) => Ok(agent
            .heap
            .create(PrimitiveObjectRecord {
                object_index: None,
                data: PrimitiveObjectData::SmallBigInt(bigint),
            })
            .into()),
        _ => Ok(Object::try_from(argument).unwrap()),
    }
}

/// ### [7.1.19 ToPropertyKey ( argument )](https://tc39.es/ecma262/#sec-topropertykey)
pub(crate) fn to_property_key<'a, 'gc>(
    agent: &mut Agent,
    argument: impl Copy + Into<Value<'a>>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, PropertyKey<'gc>> {
    // Note: Fast path and non-standard special case combined. Usually the
    // argument is already a valid property key. We also need to parse integer
    // strings back into integer property keys.
    if let Some(simple_result) = to_property_key_simple(agent, argument, gc.nogc()) {
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
pub(crate) fn to_property_key_simple<'a, 'gc>(
    agent: &Agent,
    argument: impl Into<Value<'a>>,
    gc: NoGcScope<'gc, '_>,
) -> Option<PropertyKey<'gc>> {
    let argument = argument.into().bind(gc);
    match argument {
        Value::String(_) | Value::SmallString(_) => {
            let (str, string_key) = match &argument {
                Value::String(x) => (x.get(agent).as_wtf8(), PropertyKey::String(*x)),
                Value::SmallString(x) => (x.as_wtf8(), PropertyKey::SmallString(*x)),
                _ => unreachable!(),
            };
            if let Some(key) = parse_wtf8_to_integer_property_key(str) {
                Some(key)
            } else {
                Some(string_key)
            }
        }
        Value::Integer(x) => Some(PropertyKey::Integer(x)),
        Value::SmallF64(x) if x.into_f64() == -0.0 => Some(PropertyKey::Integer(0.into())),
        Value::Symbol(x) => Some(PropertyKey::Symbol(x)),
        Value::SmallBigInt(x)
            if (SmallInteger::MIN..=SmallInteger::MAX).contains(&x.into_i64()) =>
        {
            Some(PropertyKey::Integer(
                // SAFETY: Range check performed above.
                unsafe { SmallInteger::from_small_bigint_unchecked(x) },
            ))
        }
        Value::Undefined => Some(PropertyKey::from(BUILTIN_STRING_MEMORY.undefined)),
        Value::Null => Some(PropertyKey::from(BUILTIN_STRING_MEMORY.null)),
        Value::Boolean(bool) => {
            if bool {
                Some(PropertyKey::from(BUILTIN_STRING_MEMORY.r#true))
            } else {
                Some(PropertyKey::from(BUILTIN_STRING_MEMORY.r#false))
            }
        }
        _ => None,
    }
}

pub(crate) fn to_property_key_complex<'a, 'gc>(
    agent: &mut Agent,
    argument: impl Into<Value<'a>>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, PropertyKey<'gc>> {
    // 1. Let key be ? ToPrimitive(argument, hint String).
    let key = to_primitive(agent, argument, Some(PreferredType::String), gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let key = key.bind(gc);

    Ok(to_property_key_primitive(agent, key, gc))
}

pub(crate) fn to_property_key_primitive<'a>(
    agent: &mut Agent,
    primitive: Primitive<'a>,
    gc: NoGcScope<'a, '_>,
) -> PropertyKey<'a> {
    // 2. If Type(key) is Symbol, then
    //    a. Return key.
    // Note: We can reuse the fast path and non-standard special case handler:
    // If the property key was an object, it is now a primitive. We need to do
    // our non-standard parsing of integer strings back into integer property
    // keys here as well.
    if let Some(key) = to_property_key_simple(agent, primitive, gc) {
        key
    } else {
        // Key was still not simple: This mean it's a heap allocated f64,
        // BigInt, or non-negative-zero f32: These should never be safe
        // integers and thus will never be PropertyKey::Integer after
        // stringifying.

        // 3. Return ! ToString(key).
        to_string_primitive(agent, primitive, gc).unwrap().into()
    }
}

pub(crate) fn parse_wtf8_to_integer_property_key(str: &Wtf8) -> Option<PropertyKey<'static>> {
    if let Some(str) = str.as_str() {
        parse_string_to_integer_property_key(str)
    } else {
        // Note: invalid UTF-8 cannot parse into an integer.
        None
    }
}
pub(crate) fn parse_string_to_integer_property_key(str: &str) -> Option<PropertyKey<'static>> {
    // i64::from_string will accept eg. 0123 as 123 but JS property keys do
    // not agree. Hence, only "0" can start with "0", all other integer
    // keys must start with one of "1".."9".
    if str == "0" {
        Some(0.into())
    } else if str == "-0" {
        None
    } else if !str.is_empty()
        && (str.starts_with('-') || (b'1'..=b'9').contains(&str.as_bytes()[0]))
        && let Ok(result) = str.parse::<i64>()
        && (SmallInteger::MIN..=SmallInteger::MAX).contains(&result)
    {
        Some(SmallInteger::try_from(result).unwrap().into())
    } else {
        None
    }
}

/// ### [7.1.20 ToLength ( argument )](https://tc39.es/ecma262/#sec-tolength)
pub(crate) fn to_length<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, i64> {
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let len be ? ToIntegerOrInfinity(argument).
    let len = to_integer_or_infinity(agent, argument, gc)?.into_i64();

    // 2. If len ≤ 0, return +0𝔽.
    if len <= 0 {
        return Ok(0);
    }

    // 3. Return 𝔽(min(len, 2**53 - 1)).
    Ok(len.min(SmallInteger::MAX))
}

/// ### [7.1.20 ToLength ( argument )](https://tc39.es/ecma262/#sec-tolength)
pub(crate) fn try_to_length<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope<'a, '_>,
) -> TryResult<'a, i64> {
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let len be ? ToIntegerOrInfinity(argument).
    let len = try_to_integer_or_infinity(agent, argument, gc)?.into_i64();

    // 2. If len ≤ 0, return +0𝔽.
    if len <= 0 {
        return TryResult::Continue(0);
    }

    // 3. Return 𝔽(min(len, 2**53 - 1)).
    TryResult::Continue(len.min(SmallInteger::MAX))
}

/// ### [7.1.21 CanonicalNumericIndexString ( argument )](https://tc39.es/ecma262/#sec-canonicalnumericindexstring)
#[cfg(feature = "array-buffer")]
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
    let n = to_number_primitive(agent, argument.into(), gc).unwrap();

    // 3. If ! ToString(n) is argument, return n.
    if to_string_primitive(agent, n.into(), gc).unwrap() == argument {
        return Some(n);
    }

    // 4. Return undefined.
    None
}

/// 2. If integer is not in the inclusive interval from 0 to 2**53 - 1, throw a
///    RangeError exception.
#[inline]
pub(crate) fn validate_index<'a>(
    agent: &mut Agent,
    value: i64,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, u64> {
    if !(0..=(SmallInteger::MAX)).contains(&value) {
        return throw_index_out_of_range(agent, gc).map(|_| unreachable!());
    }
    Ok(value as u64)
}

#[inline(never)]
#[cold]
fn throw_index_out_of_range<'gc>(
    agent: &mut Agent,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, Infallible> {
    Err(agent.throw_exception_with_static_message(
        ExceptionType::RangeError,
        "Index is out of range",
        gc,
    ))
}

/// ### [7.1.22 ToIndex ( value )](https://tc39.es/ecma262/#sec-toindex)
pub(crate) fn to_index<'a>(
    agent: &mut Agent,
    argument: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, u64> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(integer) = argument {
        return validate_index(agent, integer.into_i64(), gc.into_nogc());
    }
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let integer be ? ToIntegerOrInfinity(value).
    let integer = to_integer_or_infinity(agent, argument, gc.reborrow())
        .unbind()?
        .into_i64();

    // 2. If integer is not in the inclusive interval from 0 to 2**53 - 1, throw a RangeError exception.
    // 3. Return integer.
    validate_index(agent, integer, gc.into_nogc())
}

/// ### [7.1.22 ToIndex ( value )](https://tc39.es/ecma262/#sec-toindex)
#[cfg(feature = "array-buffer")]
pub(crate) fn try_to_index<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope<'a, '_>,
) -> TryResult<'a, u64> {
    // Fast path: A safe integer is already an integer.
    if let Value::Integer(integer) = argument {
        return js_result_into_try(validate_index(agent, integer.into_i64(), gc));
    }
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let integer be ? ToIntegerOrInfinity(value).
    let integer = try_to_integer_or_infinity(agent, argument, gc)?.into_i64();

    // 2. If integer is not in the inclusive interval from 0 to 2**53 - 1, throw a RangeError exception.
    // 3. Return integer.
    js_result_into_try(validate_index(agent, integer, gc))
}

/// Helper function to check if a `char` is trimmable.
///
/// Copied from Boa JS engine. Source https://github.com/boa-dev/boa/blob/183e763c32710e4e3ea83ba762cf815b7a89cd1f/core/string/src/lib.rs#L51
///
/// Copyright (c) 2019 Jason Williams
pub(crate) const fn is_trimmable_whitespace(c: char) -> bool {
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
