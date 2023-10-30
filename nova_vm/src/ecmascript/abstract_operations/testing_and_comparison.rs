//! ## [7.2 Testing and Comparison Operations](https://tc39.es/ecma262/#sec-testing-and-comparison-operations)

use crate::ecmascript::{
    execution::{agent::JsError, Agent, JsResult},
    types::{Number, Value},
};

/// ### [7.2.1 RequireObjectCoercible ( argument )](https://tc39.es/ecma262/#sec-requireobjectcoercible)
///
/// The abstract operation RequireObjectCoercible takes argument argument (an
/// ECMAScript language value) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. It throws an
/// error if argument is a value that cannot be converted to an Object using
/// ToObject. It is defined by [Table 14](https://tc39.es/ecma262/#table-requireobjectcoercible-results):
pub(crate) fn require_object_coercible(_agent: &mut Agent, argument: Value) -> JsResult<Value> {
    if argument.is_undefined() || argument.is_null() {
        Err(JsError {})
    } else {
        Ok(argument)
    }
}

/// ### [7.2.2 IsArray ( argument )](https://tc39.es/ecma262/#sec-isarray)
///
/// The abstract operation IsArray takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing a Boolean
/// or a throw completion.
pub(crate) fn is_array(_agent: &Agent, argument: Value) -> JsResult<bool> {
    // 1. If argument is not an Object, return false.
    // 2. If argument is an Array exotic object, return true.
    Ok(matches!(argument, Value::Array(_)))
    // TODO: Proxy
    // 3. If argument is a Proxy exotic object, then
    // a. Perform ? ValidateNonRevokedProxy(argument).
    // b. Let proxyTarget be argument.[[ProxyTarget]].
    // c. Return ? IsArray(proxyTarget).
    // 4. Return false.
}

/// ### [7.2.3 IsCallable ( argument )](https://tc39.es/ecma262/#sec-iscallable)
///
/// The abstract operation IsCallable takes argument argument (an ECMAScript
/// language value) and returns a Boolean. It determines if argument is a
/// callable function with a [[Call]] internal method.
pub(crate) fn is_callable(argument: Value) -> bool {
    // 1. If argument is not an Object, return false.
    // 2. If argument has a [[Call]] internal method, return true.
    // 3. Return false.
    matches!(argument, Value::Function(_))
}

pub(crate) fn is_same_type<V1: Copy + Into<Value>, V2: Copy + Into<Value>>(x: V1, y: V2) -> bool {
    (x.into().is_undefined() && y.into().is_undefined())
        || (x.into().is_null() && y.into().is_null())
        || (x.into().is_boolean() && y.into().is_boolean())
        || (x.into().is_string() && y.into().is_string())
        || (x.into().is_symbol() && y.into().is_symbol())
        || (x.into().is_number() && y.into().is_number())
        || (x.into().is_object() && y.into().is_object())
}

/// 7.2.10 SameValue ( x, y )
/// https://tc39.es/ecma262/#sec-samevalue
pub(crate) fn same_value<V1: Copy + Into<Value>, V2: Copy + Into<Value>>(
    agent: &mut Agent,
    x: V1,
    y: V2,
) -> bool {
    // 1. If Type(x) is not Type(y), return false.
    if !is_same_type(x, y) {
        return false;
    }

    // 2. If x is a Number, then
    if let (Ok(x), Ok(y)) = (Number::try_from(x.into()), Number::try_from(y.into())) {
        // a. Return Number::sameValue(x, y).
        return x.same_value(agent, y);
    }

    // 3. Return SameValueNonNumber(x, y).
    let x: Value = x.into();
    let y: Value = y.into();
    same_value_non_number(agent, x, y)
}

/// ### [7.2.11 SameValueZero ( x, y )](https://tc39.es/ecma262/#sec-samevaluezero)
///
/// The abstract operation SameValueZero takes arguments x (an ECMAScript
/// language value) and y (an ECMAScript language value) and returns a Boolean.
/// It determines whether or not the two arguments are the same value (ignoring
/// the difference between +0𝔽 and -0𝔽). It performs the following steps when
/// called:
pub(crate) fn same_value_zero(
    agent: &mut Agent,
    x: impl Copy + Into<Value>,
    y: impl Copy + Into<Value>,
) -> bool {
    let (x, y) = (x.into(), y.into());

    // 1. If Type(x) is not Type(y), return false.
    if !is_same_type(x, y) {
        return false;
    }

    // 2. If x is a Number, then
    // NOTE: We need to convert both to a number because we use number
    // type-safety.
    if let (Ok(x), Ok(y)) = (x.to_number(agent), y.to_number(agent)) {
        // a. Return Number::sameValueZero(x, y).
        return x.same_value_zero(agent, y);
    }

    // 3. Return SameValueNonNumber(x, y).
    return same_value_non_number(agent, x, y);
}

/// 7.2.12 SameValueNonNumber ( x, y )
/// https://tc39.es/ecma262/#sec-samevaluenonnumber
pub(crate) fn same_value_non_number<T: Copy + Into<Value>>(_agent: &mut Agent, x: T, y: T) -> bool {
    let x: Value = x.into();
    let y: Value = y.into();

    // 1. Assert: Type(x) is Type(y).
    debug_assert!(is_same_type(x, y));

    // 2. If x is either null or undefined, return true.
    if x.is_null() || x.is_undefined() {
        return true;
    }

    // 3. If x is a BigInt, then
    if x.is_bigint() {
        // a. Return BigInt::equal(x, y).
        todo!();
    }

    // 4. If x is a String, then
    if x.is_string() {
        // a. If x and y have the same length and the same code units in the same positions, return true; otherwise, return false.
        todo!();
    }

    // 5. If x is a Boolean, then
    if x.is_boolean() {
        // a. If x and y are both true or both false, return true; otherwise, return false.
        return x.is_true() == y.is_true();
    }

    // 6. NOTE: All other ECMAScript language values are compared by identity.
    // 7. If x is y, return true; otherwise, return false.
    todo!()
}
