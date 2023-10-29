//! ## [7.2 Testing and Comparison Operations](https://tc39.es/ecma262/#sec-testing-and-comparison-operations)

use crate::ecmascript::{
    execution::{agent::JsError, Agent, JsResult},
    types::Value,
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
