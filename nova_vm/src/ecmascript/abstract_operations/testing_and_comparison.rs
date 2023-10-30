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
pub(crate) fn require_object_coercible(agent: &mut Agent, argument: Value) -> JsResult<Value> {
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
pub(crate) fn is_array(agent: &Agent, argument: Value) -> JsResult<bool> {
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

pub(crate) fn is_same_type(x: Value, y: Value) -> bool {
    (x.is_undefined() && y.is_undefined())
        || (x.is_null() && y.is_null())
        || (x.is_boolean() && y.is_boolean())
        || (x.is_string() && y.is_string())
        || (x.is_symbol() && y.is_symbol())
        || (x.is_number() && y.is_number())
        || (x.is_object() && y.is_object())
}

/// 7.2.10 SameValue ( x, y )
/// https://tc39.es/ecma262/#sec-samevalue
pub(crate) fn same_value(agent: &mut Agent, x: Value, y: Value) -> bool {
    // 1. If Type(x) is not Type(y), return false.
    if !is_same_type(x, y) {
        return false;
    }

    // 2. If x is a Number, then
    if let (Ok(x), Ok(y)) = (Number::try_from(x), Number::try_from(y)) {
        // a. Return Number::sameValue(x, y).
        return x.same_value(agent, y);
    }

    // 3. Return SameValueNonNumber(x, y).
    same_value_non_number(agent, x, y)
}

/// 7.2.12 SameValueNonNumber ( x, y )
/// https://tc39.es/ecma262/#sec-samevaluenonnumber
pub(crate) fn same_value_non_number(_agent: &mut Agent, x: Value, y: Value) -> bool {
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
