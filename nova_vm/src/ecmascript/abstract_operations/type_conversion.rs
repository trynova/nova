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

use crate::{
    ecmascript::{
        execution::{agent::ExceptionType, agent::JsError, Agent, JsResult},
        types::{BigInt, Number, Object, PropertyKey, String, Value},
    },
    heap::{GetHeapData, WellKnownSymbolIndexes},
    SmallInteger,
};

use super::{
    operations_on_objects::{call, call_function, get, get_method},
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
/// behaves as if the hint were NUMBER. However, objects may over-ride this
/// behaviour by defining a @@toPrimitive method. Of the objects defined in
/// this specification only Dates (see 21.4.4.45) and Symbol objects (see
/// 20.4.3.5) over-ride the default ToPrimitive behaviour. Dates treat the
/// absence of a hint as if the hint were STRING.
pub(crate) fn to_primitive(
    agent: &mut Agent,
    input: Value,
    preferred_type: Option<PreferredType>,
) -> JsResult<Value> {
    // 1. If input is an Object, then
    if let Ok(input) = Object::try_from(input) {
        // a. Let exoticToPrim be ? GetMethod(input, @@toPrimitive).
        let exotic_to_prim = get_method(
            agent,
            input.into_value(),
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToPrimitive.into()),
        )?;
        // b. If exoticToPrim is not undefined, then
        if let Some(exotic_to_prim) = exotic_to_prim {
            let hint = match preferred_type {
                // i. If preferredType is not present, then
                // 1. Let hint be "default".
                None => String::from_small_string("default"),
                // ii. Else if preferredType is STRING, then
                // 1. Let hint be "string".
                Some(PreferredType::String) => String::from_small_string("string"),
                // iii. Else,
                // 1. Assert: preferredType is NUMBER.
                // 2. Let hint be "number".
                Some(PreferredType::Number) => String::from_small_string("number"),
            };
            // iv. Let result be ? Call(exoticToPrim, input, « hint »).
            let result: Value =
                call_function(agent, exotic_to_prim, input.into(), Some(&[hint.into()]))?;
            if !result.is_object() {
                // v. If result is not an Object, return result.
                Ok(result)
            } else {
                // vi. Throw a TypeError exception.
                Err(JsError {})
            }
        } else {
            // c. If preferredType is not present, let preferredType be NUMBER.
            // d. Return ? OrdinaryToPrimitive(input, preferredType).
            ordinary_to_primitive(
                agent,
                input,
                preferred_type.unwrap_or(PreferredType::Number),
            )
        }
    } else {
        // 2. Return input.
        Ok(input)
    }
}

/// ### [7.1.1.1 OrdinaryToPrimitive ( O, hint )](https://tc39.es/ecma262/#sec-ordinarytoprimitive)
///
/// The abstract operation OrdinaryToPrimitive takes arguments O (an Object)
/// and hint (STRING or NUMBER) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion.
pub(crate) fn ordinary_to_primitive(
    agent: &mut Agent,
    o: Object,
    hint: PreferredType,
) -> JsResult<Value> {
    let to_string_key = PropertyKey::from(String::from_str(agent, "toString"));
    let value_of_key = PropertyKey::from(String::from_small_string("valueOf"));
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
    for name in method_names {
        // a. Let method be ? Get(O, name).
        let method = get(agent, o, name)?;
        // b. If IsCallable(method) is true, then
        if is_callable(method) {
            // i. Let result be ? Call(method, O).
            let result: Value = call(agent, method, o.into(), None)?;
            // ii. If result is not an Object, return result.
            if !result.is_object() {
                return Ok(result);
            }
        }
    }
    // 4. Throw a TypeError exception.
    Err(JsError {})
}

/// ### [7.1.2 ToBoolean ( argument )](https://tc39.es/ecma262/#sec-toboolean)
pub(crate) fn to_boolean(agent: &mut Agent, argument: Value) -> bool {
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
pub(crate) fn to_numeric(agent: &mut Agent, value: Value) -> JsResult<Value> {
    // 1. Let primValue be ? ToPrimitive(value, number).
    let prim_value = to_primitive(agent, value, Some(PreferredType::Number))?;

    // 2. If primValue is a BigInt, return primValue.
    if prim_value.is_bigint() {
        return Ok(prim_value);
    }

    // 3. Return ? ToNumber(primValue).
    to_number(agent, value).map(|n| n.into_value())
}

/// ### [7.1.4 ToNumber ( argument )](https://tc39.es/ecma262/#sec-tonumber)
pub(crate) fn to_number(agent: &mut Agent, argument: Value) -> JsResult<Number> {
    // 1. If argument is a Number, return argument.
    if let Ok(argument) = Number::try_from(argument) {
        return Ok(argument);
    }

    // 2. If argument is either a Symbol or a BigInt, throw a TypeError exception.
    if argument.is_symbol() {
        return Err(
            agent.throw_exception(ExceptionType::TypeError, "cannot convert symbol to number")
        );
    }
    if argument.is_bigint() {
        return Err(
            agent.throw_exception(ExceptionType::TypeError, "cannot convert bigint to number")
        );
    }

    // 3. If argument is undefined, return NaN.
    if argument.is_undefined() {
        return Ok(Number::nan());
    }

    // 4. If argument is either null or false, return +0𝔽.
    if argument.is_null() || argument.is_false() {
        return Ok(Number::from(0));
    }

    // 5. If argument is true, return 1𝔽.
    if argument.is_true() {
        return Ok(Number::from(1));
    }

    // 6. If argument is a String, return StringToNumber(argument).
    if argument.is_string() {
        todo!("implement StringToNumber");
    }

    // 7. Assert: argument is an Object.
    debug_assert!(argument.is_object());

    // 8. Let primValue be ? ToPrimitive(argument, number).
    let prim_value = to_primitive(agent, argument, Some(PreferredType::Number))?;

    // 9. Assert: primValue is not an Object.
    debug_assert!(!prim_value.is_object());

    // 10. Return ? ToNumber(primValue).
    to_number(agent, prim_value)
}

/// ### [7.1.5 ToIntegerOrInfinity ( argument )](https://tc39.es/ecma262/#sec-tointegerorinfinity)
// TODO: Should we add another [`Value`] newtype for IntegerOrInfinity?
pub(crate) fn to_integer_or_infinity(agent: &mut Agent, argument: Value) -> JsResult<Number> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is one of NaN, +0𝔽, or -0𝔽, return 0.
    if number.is_nan(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return Ok(Number::pos_zero());
    }

    // 3. If number is +∞𝔽, return +∞.
    if number.is_pos_infinity(agent) {
        return Ok(Number::pos_inf());
    }

    // 4. If number is -∞𝔽, return -∞.
    if number.is_neg_infinity(agent) {
        return Ok(Number::neg_inf());
    }

    // 5. Return truncate(ℝ(number)).
    Ok(number.truncate(agent))
}

/// ### [7.1.6 ToInt32 ( argument )](https://tc39.es/ecma262/#sec-toint32)
pub(crate) fn to_int32(agent: &mut Agent, argument: Value) -> JsResult<i32> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return Ok(0);
    }

    // 3. Let int be truncate(ℝ(number)).
    let int = number.truncate(agent).into_f64(agent);

    // 4. Let int32bit be int modulo 2^32.
    let int32bit = int % 2f64.powi(32);

    // 5. If int32bit ≥ 2^31, return 𝔽(int32bit - 2^32); otherwise return 𝔽(int32bit).
    Ok(if int32bit >= 2f64.powi(32) {
        int32bit - 2f64.powi(32)
    } else {
        int32bit
    } as i32)
}

/// ### [7.1.7 ToUint32 ( argument )](https://tc39.es/ecma262/#sec-touint32)
pub(crate) fn to_uint32(agent: &mut Agent, argument: Value) -> JsResult<u32> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return Ok(0);
    }

    // 3. Let int be truncate(ℝ(number)).
    let int = number.truncate(agent).into_f64(agent);

    // 4. Let int32bit be int modulo 2^32.
    let int32bit = int % 2f64.powi(32);

    // 5. Return 𝔽(int32bit).
    Ok(int32bit as u32)
}

/// ### [7.1.8 ToInt16 ( argument )](https://tc39.es/ecma262/#sec-toint16)
pub(crate) fn to_int16(agent: &mut Agent, argument: Value) -> JsResult<i16> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return Ok(0);
    }

    // 3. Let int be truncate(ℝ(number)).
    let int = number.truncate(agent).into_f64(agent);

    // 4. Let int16bit be int modulo 2^16.
    let int16bit = int % 2f64.powi(16);

    // 5. If int16bit ≥ 2^15, return 𝔽(int16bit - 2^16); otherwise return 𝔽(int16bit).
    Ok(if int16bit >= 2f64.powi(15) {
        int16bit - 2f64.powi(16)
    } else {
        int16bit
    } as i16)
}

/// ### [7.1.9 ToUint16 ( argument )](https://tc39.es/ecma262/#sec-touint16)
pub(crate) fn to_uint16(agent: &mut Agent, argument: Value) -> JsResult<i16> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return Ok(0);
    }

    // 3. Let int be truncate(ℝ(number)).
    let int = number.truncate(agent).into_f64(agent);

    // 4. Let int16bit be int modulo 2^16.
    let int16bit = int % 2f64.powi(16);

    // Return 𝔽(int16bit).
    Ok(int16bit as i16)
}

/// ### [7.1.10 ToInt8 ( argument )](https://tc39.es/ecma262/#sec-toint8)
pub(crate) fn to_int8(agent: &mut Agent, argument: Value) -> JsResult<i8> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return Ok(0);
    }

    // 3. Let int be truncate(ℝ(number)).
    let int = number.truncate(agent).into_f64(agent);

    // 4. Let int8bit be int modulo 2^8.
    let int8bit = int % 2f64.powi(8);

    // 5. If int8bit ≥ 2^7, return 𝔽(int8bit - 2^8); otherwise return 𝔽(int8bit).
    Ok(if int8bit >= 2f64.powi(7) {
        int8bit - 2f64.powi(8)
    } else {
        int8bit
    } as i8)
}

/// ### [7.1.11 ToUint8 ( argument )](https://tc39.es/ecma262/#sec-touint8)
pub(crate) fn to_uint8(agent: &mut Agent, argument: Value) -> JsResult<u8> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is not finite or number is either +0𝔽 or -0𝔽, return +0𝔽.
    if !number.is_finite(agent) || number.is_pos_zero(agent) || number.is_neg_zero(agent) {
        return Ok(0);
    }

    // 3. Let int be truncate(ℝ(number)).
    let int = number.truncate(agent).into_f64(agent);

    // 4. Let int8bit be int modulo 2^8.
    let int8bit = int % 2f64.powi(8);

    // 5. Return 𝔽(int8bit).
    Ok(int8bit as u8)
}

/// ### [7.1.12 ToUint8Clamp ( argument )](https://tc39.es/ecma262/#sec-touint8clamp)
pub(crate) fn to_uint8_clamp(agent: &mut Agent, argument: Value) -> JsResult<u8> {
    // 1. Let number be ? ToNumber(argument).
    let number = to_number(agent, argument)?;

    // 2. If number is NaN, return +0𝔽.
    if number.is_nan(agent) {
        return Ok(0);
    }

    // 3. Let mv be the extended mathematical value of number.
    // TODO: Is there a better way?
    let mv = number.into_f64(agent);

    // 4. Let clamped be the result of clamping mv between 0 and 255.
    let clamped = mv.clamp(0.0, 255.0);

    // 5. Let f be floor(clamped).
    let f = clamped.floor();

    Ok(
        // 6. If clamped < f + 0.5, return 𝔽(f).
        if clamped < f + 0.5 {
            f as u8
        }
        // 7. If clamped > f + 0.5, return 𝔽(f + 1).
        else if clamped > f + 0.5 {
            f as u8 + 1
        }
        // 8. If f is even, return 𝔽(f). Otherwise, return 𝔽(f + 1).
        else if f % 2.0 == 0.0 {
            f as u8
        } else {
            f as u8 + 1
        },
    )
}

/// ### [7.1.13 ToBigInt ( argument )](https://tc39.es/ecma262/#sec-tobigint)
pub(crate) fn to_big_int(agent: &mut Agent, argument: Value) -> JsResult<BigInt> {
    // 1. Let prim be ? ToPrimitive(argument, number).
    let _prim = to_primitive(agent, argument, Some(PreferredType::Number))?;

    // 2. Return the value that prim corresponds to in Table 12.
    todo!()
}

/// ### [7.1.14 StringToBigInt ( str )](https://tc39.es/ecma262/#sec-stringtobigint)
pub(crate) fn string_to_big_int(_agent: &mut Agent, _argument: Value) -> Option<Value> {
    // 1. Let text be StringToCodePoints(str).
    // 2. Let literal be ParseText(text, StringIntegerLiteral).
    // 3. If literal is a List of errors, return undefined.
    // 4. Let mv be the MV of literal.
    // 5. Assert: mv is an integer.
    // 6. Return ℤ(mv).

    todo!("string_to_big_int: Implement BigInts")
}

/// ### [7.1.17 ToString ( argument )](https://tc39.es/ecma262/#sec-tostring)
pub(crate) fn to_string(_agent: &mut Agent, _argument: Value) -> JsResult<String> {
    // TODO: 1. If argument is a String, return argument.
    // 2. If argument is a Symbol, throw a TypeError exception.
    // 3. If argument is undefined, return "undefined".
    // 4. If argument is null, return "null".
    // 5. If argument is true, return "true".
    // 6. If argument is false, return "false".
    // 7. If argument is a Number, return Number::toString(argument, 10).
    // 8. If argument is a BigInt, return BigInt::toString(argument, 10).
    // 9. Assert: argument is an Object.
    // 10. Let primValue be ? ToPrimitive(argument, string).
    // 11. Assert: primValue is not an Object.
    // 12. Return ? ToString(primValue).

    todo!()
}

/// ### [7.1.18 ToObject ( argument )](https://tc39.es/ecma262/#sec-toobject)
///
/// The abstract operation ToObject takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing an Object
/// or a throw completion. It converts argument to a value of type Object
/// according to [Table 13](https://tc39.es/ecma262/#table-toobject-conversions):
pub(crate) fn to_object(_agent: &mut Agent, argument: Value) -> JsResult<Object> {
    match argument {
        Value::Undefined | Value::Null => Err(JsError {}),
        // Return a new Boolean object whose [[BooleanData]] internal slot is set to argument.
        Value::Boolean(_) => todo!("BooleanObject"),
        // Return a new String object whose [[StringData]] internal slot is set to argument.
        Value::String(_) => todo!("StringObject"),
        Value::SmallString(_) => todo!("StringObject"),
        // Return a new Symbol object whose [[SymbolnData]] internal slot is set to argument.
        Value::Symbol(_) => todo!("SymbolObject"),
        // Return a new Number object whose [[NumberData]] internal slot is set to argument.
        Value::Number(_) => todo!("NumberObject"),
        Value::Integer(_) => todo!("NumberObject"),
        Value::Float(_) => todo!("NumberObject"),
        // Return a new BigInt object whose [[BigIntData]] internal slot is set to argument.
        Value::BigInt(_) => todo!("BigIntObject"),
        Value::SmallBigInt(_) => todo!("BigIntObject"),
        _ => Ok(Object::try_from(argument).unwrap()),
    }
}

/// ### [7.1.19 ToPropertyKey ( argument )](https://tc39.es/ecma262/#sec-topropertykey)
pub(crate) fn to_property_key(agent: &mut Agent, argument: Value) -> JsResult<PropertyKey> {
    // 1. Let key be ? ToPrimitive(argument, hint String).
    let key = to_primitive(agent, argument, Some(PreferredType::String))?;

    // 2. If Type(key) is Symbol, then
    //    a. Return key.
    // NOTE: This handles Symbols and other primitives because we use niche
    // specializations for PropertyKey (e.g. integer indexes for arrays).
    if let Ok(property_key) = PropertyKey::try_from(key) {
        return Ok(property_key);
    }

    // 3. Return ! ToString(key).
    Ok(to_string(agent, key).unwrap().into())
}

/// ### [7.1.20 ToLength ( argument )](https://tc39.es/ecma262/#sec-tolength)
pub(crate) fn to_length(agent: &mut Agent, argument: Value) -> JsResult<i64> {
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let len be ? ToIntegerOrInfinity(argument).
    let len = to_integer_or_infinity(agent, argument)?;

    // 2. If len ≤ 0, return +0𝔽.
    if match len {
        Number::Integer(n) => n.into_i64() <= 0,
        Number::Float(n) => n <= 0.0,
        Number::Number(n) => *agent.heap.get(n) <= 0.0,
    } {
        return Ok(0);
    }

    // 3. Return 𝔽(min(len, 2**53 - 1)).
    Ok(match len {
        Number::Integer(n) => n.into_i64().min(SmallInteger::MAX_NUMBER),
        Number::Float(n) => n.min(SmallInteger::MAX_NUMBER as f32) as i64,
        Number::Number(n) => agent.heap.get(n).min(SmallInteger::MAX_NUMBER as f64) as i64,
    })
}

/// ### [7.1.21 CanonicalNumericIndexString ( argument )](https://tc39.es/ecma262/#sec-canonicalnumericindexstring)
pub(crate) fn canonical_numeric_index_string(
    agent: &mut Agent,
    argument: String,
) -> Option<Number> {
    // 1. If argument is "-0", return -0𝔽.
    if argument == String::from_small_string("-0") {
        return Some((-0.0).into());
    }

    // 2. Let n be ! ToNumber(argument).
    let n = to_number(agent, argument.into()).unwrap();

    // 3. If ! ToString(n) is argument, return n.
    if to_string(agent, n.into()).unwrap() == argument {
        return Some(n);
    }

    // 4. Return undefined.
    None
}

/// ### [7.1.22 ToIndex ( value )](https://tc39.es/ecma262/#sec-toindex)
pub(crate) fn to_index(agent: &mut Agent, argument: Value) -> JsResult<i64> {
    // TODO: This can be heavily optimized by inlining `to_integer_or_infinity`.

    // 1. Let integer be ? ToIntegerOrInfinity(value).
    let integer = to_integer_or_infinity(agent, argument)?;

    // 2. If integer is not in the inclusive interval from 0 to 2**53 - 1, throw a RangeError exception.
    let integer = if let Number::Integer(n) = integer {
        let integer = n.into_i64();
        if !(0..=(SmallInteger::MAX_NUMBER)).contains(&integer) {
            return Err(agent.throw_exception(ExceptionType::RangeError, "Result is out of range"));
        }
        integer
    } else {
        // to_integer_or_infinity returns +0, +Infinity, -Infinity, or an integer.
        return Err(agent.throw_exception(ExceptionType::RangeError, "Result is out of range"));
    };

    // 3. Return integer.
    Ok(integer)
}
