//! ## [7.2 Testing and Comparison Operations](https://tc39.es/ecma262/#sec-testing-and-comparison-operations)

use crate::ecmascript::{
    execution::{agent::JsError, Agent, JsResult},
    types::{bigint::BigInt, Number, Value},
};

use super::type_conversion::{string_to_big_int, to_number, to_primitive, PreferredType};

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
    matches!(
        argument,
        Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_)
    )
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

/// ### [7.2.6 IsIntegralNumber ( argument )](https://tc39.es/ecma262/#sec-isintegralnumber)
pub(crate) fn is_integral_number(agent: &mut Agent, argument: impl Copy + Into<Value>) -> bool {
    let argument = argument.into();

    // OPTIMIZATION: If the number is a small integer, then know that it must be
    // an integral number.
    if let Value::Integer(_) = argument {
        return true;
    }

    // 1. If argument is not a Number, return false.
    let Ok(argument) = Number::try_from(argument) else {
        return false;
    };

    // 2. If argument is not finite, return false.
    if !argument.is_finite(agent) {
        return false;
    }

    // 3. If truncate(ℝ(argument)) ≠ ℝ(argument), return false.
    // 4. Return true.
    // NOTE: Checking if the fractional component is 0.0 is the same as the
    // specification's operation.
    argument.into_value().to_real(agent).unwrap().fract() == 0.0
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
    same_value_non_number(agent, x, y)
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

/// [7.2.13 IsLessThan ( x, y, LeftFirst )](https://tc39.es/ecma262/#sec-islessthan)
///
/// The abstract operation IsLessThan takes arguments x (an ECMAScript language
/// value), y (an ECMAScript language value), and LeftFirst (a Boolean) and
/// returns either a normal completion containing either a Boolean or undefined,
/// or a throw completion. It provides the semantics for the comparison x < y,
/// returning true, false, or undefined (which indicates that at least one
/// operand is NaN). The LeftFirst flag is used to control the order in which
/// operations with potentially visible side-effects are performed upon x and y.
/// It is necessary because ECMAScript specifies left to right evaluation of
/// expressions. If LeftFirst is true, the x parameter corresponds to an
/// expression that occurs to the left of the y parameter's corresponding
/// expression. If LeftFirst is false, the reverse is the case and operations
/// must be performed upon y before x. It performs the following steps when
/// called:
pub(crate) fn is_less_than<const LEFT_FIRST: bool>(
    agent: &mut Agent,
    x: impl Into<Value> + Copy,
    y: impl Into<Value> + Copy,
) -> JsResult<Option<bool>> {
    // 1. If LeftFirst is true, then
    let (px, py) = if LEFT_FIRST {
        // a. Let px be ? ToPrimitive(x, NUMBER).
        let px = to_primitive(agent, x.into(), Some(PreferredType::Number))?;

        // b. Let py be ? ToPrimitive(y, NUMBER).
        let py = to_primitive(agent, y.into(), Some(PreferredType::Number))?;

        (px, py)
    }
    // 2. Else,
    else {
        // a. NOTE: The order of evaluation needs to be reversed to preserve left to right evaluation.
        // b. Let py be ? ToPrimitive(y, NUMBER).
        let py = to_primitive(agent, y.into(), Some(PreferredType::Number))?;

        // c. Let px be ? ToPrimitive(x, NUMBER).
        let px = to_primitive(agent, x.into(), Some(PreferredType::Number))?;

        (px, py)
    };

    // 3. If px is a String and py is a String, then
    if px.is_string() && py.is_string() {
        todo!("Finish this")
        // a. Let lx be the length of px.
        // b. Let ly be the length of py.
        // c. For each integer i such that 0 ≤ i < min(lx, ly), in ascending order, do
        // i. Let cx be the numeric value of the code unit at index i within px.
        // ii. Let cy be the numeric value of the code unit at index i within py.
        // iii. If cx < cy, return true.
        // iv. If cx > cy, return false.
        // d. If lx < ly, return true. Otherwise, return false.
    }
    // 4. Else,
    else {
        // a. If px is a BigInt and py is a String, then
        if px.is_bigint() && py.is_string() {
            todo!("Finish this")
            // i. Let ny be StringToBigInt(py).
            // ii. If ny is undefined, return undefined.
            // iii. Return BigInt::lessThan(px, ny).
        }

        // b. If px is a String and py is a BigInt, then
        if px.is_string() && py.is_bigint() {
            todo!("Finish this")
            // i. Let nx be StringToBigInt(px).
            // ii. If nx is undefined, return undefined.
            // iii. Return BigInt::lessThan(nx, py).
        }

        // c. NOTE: Because px and py are primitive values, evaluation order is not important.
        // d. Let nx be ? ToNumeric(px).
        let nx = px.to_numeric(agent)?;

        // e. Let ny be ? ToNumeric(py).
        let ny = py.to_numeric(agent)?;

        // f. If Type(nx) is Type(ny), then
        if is_same_type(nx, ny) {
            // i. If nx is a Number, then
            if nx.is_number() {
                // 1. Return Number::lessThan(nx, ny).
                let nx = nx.to_number(agent)?;
                let ny = ny.to_number(agent)?;
                return Ok(nx.less_than(agent, ny));
            }
            // ii. Else,
            else {
                // 1. Assert: nx is a BigInt.
                assert!(nx.is_bigint());

                // 2. Return BigInt::lessThan(nx, ny).
                let nx = nx.to_bigint(agent)?;
                let ny = ny.to_bigint(agent)?;
                return Ok(Some(BigInt::less_than(agent, nx, ny)));
            }
        }

        // g. Assert: nx is a BigInt and ny is a Number, or nx is a Number and ny is a BigInt.
        assert!(nx.is_bigint() && ny.is_number() || nx.is_number() && ny.is_bigint());

        // h. If nx or ny is NaN, return undefined.
        if nx.is_nan(agent) || ny.is_nan(agent) {
            return Ok(None);
        }

        // i. If nx is -∞𝔽 or ny is +∞𝔽, return true.
        if nx.is_neg_infinity(agent) || ny.is_pos_infinity(agent) {
            return Ok(Some(true));
        }

        // j. If nx is +∞𝔽 or ny is -∞𝔽, return false.
        if nx.is_pos_infinity(agent) || ny.is_neg_infinity(agent) {
            return Ok(Some(false));
        }

        // k. If ℝ(nx) < ℝ(ny), return true; otherwise return false.
        let rnx = nx.to_real(agent)?;
        let rny = nx.to_real(agent)?;
        Ok(Some(rnx < rny))
    }
}

/// [7.2.14 IsLooselyEqual ( x, y )](https://tc39.es/ecma262/#sec-islooselyequal)
///
/// The abstract operation IsLooselyEqual takes arguments x (an ECMAScript
/// language value) and y (an ECMAScript language value) and returns either a
/// normal completion containing a Boolean or a throw completion. It provides
/// the semantics for the == operator.
pub(crate) fn is_loosely_equal(
    agent: &mut Agent,
    x: impl Into<Value> + Copy,
    y: impl Into<Value> + Copy,
) -> JsResult<bool> {
    let (x, y) = (x.into(), y.into());

    // 1. If Type(x) is Type(y), then
    if is_same_type(x, y) {
        // a. Return IsStrictlyEqual(x, y).
        return Ok(is_strictly_equal(agent, x, y));
    }

    // 2. If x is null and y is undefined, return true.
    // 3. If x is undefined and y is null, return true.
    if (x.is_null() && y.is_undefined()) || (x.is_undefined() && y.is_null()) {
        return Ok(true);
    }

    // TODO:
    // 4. Perform the following steps:
    // a. If x is an Object, x has an [[IsHTMLDDA]] internal slot, and y is either undefined or null, return true.
    // b. If x is either undefined or null, y is an Object, and y has an [[IsHTMLDDA]] internal slot, return true.

    // 5. If x is a Number and y is a String, return ! IsLooselyEqual(x, ! ToNumber(y)).
    if x.is_number() && y.is_string() {
        let y = to_number(agent, y).unwrap();
        return Ok(is_loosely_equal(agent, x, y).unwrap());
    }

    // 6. If x is a String and y is a Number, return ! IsLooselyEqual(! ToNumber(x), y).
    if x.is_string() && y.is_number() {
        let x = to_number(agent, x).unwrap();
        return Ok(is_loosely_equal(agent, x, y).unwrap());
    }

    // 7. If x is a BigInt and y is a String, then
    if x.is_bigint() && y.is_string() {
        // a. Let n be StringToBigInt(y).
        // b. If n is undefined, return false.
        if let Some(n) = string_to_big_int(agent, y) {
            // c. Return ! IsLooselyEqual(x, n).
            return Ok(is_loosely_equal(agent, x, n).unwrap());
        } else {
            return Ok(false);
        }
    }

    // 8. If x is a String and y is a BigInt, return ! IsLooselyEqual(y, x).
    if x.is_string() && y.is_bigint() {
        return Ok(is_loosely_equal(agent, y, x).unwrap());
    }

    // 9. If x is a Boolean, return ! IsLooselyEqual(! ToNumber(x), y).
    if x.is_boolean() {
        let x = to_number(agent, x).unwrap();
        return Ok(is_loosely_equal(agent, x, y).unwrap());
    }

    // 10. If y is a Boolean, return ! IsLooselyEqual(x, ! ToNumber(y)).
    if y.is_boolean() {
        let y = to_number(agent, y).unwrap();
        return Ok(is_loosely_equal(agent, x, y).unwrap());
    }

    // 11. If x is either a String, a Number, a BigInt, or a Symbol and y is an Object, return ! IsLooselyEqual(x, ? ToPrimitive(y)).
    if (x.is_string() || x.is_number() || x.is_bigint() || x.is_symbol()) && y.is_object() {
        let y = to_primitive(agent, y, None)?;
        return Ok(is_loosely_equal(agent, x, y).unwrap());
    }

    // 12. If x is an Object and y is either a String, a Number, a BigInt, or a Symbol, return ! IsLooselyEqual(? ToPrimitive(x), y).
    if x.is_object() && (y.is_string() || y.is_number() || y.is_bigint() || y.is_symbol()) {
        let x = to_primitive(agent, x, None)?;
        return Ok(is_loosely_equal(agent, x, y).unwrap());
    }

    // 13. If x is a BigInt and y is a Number, or if x is a Number and y is a BigInt, then
    if let Some(xy) = if x.is_bigint() {
        y.to_number(agent).ok()
    } else if y.is_bigint() {
        x.to_number(agent).ok()
    } else {
        None
    } {
        // a. If x is not finite or y is not finite, return false.
        if !xy.is_finite(agent) {
            return Ok(false);
        }

        // b. If ℝ(x) = ℝ(y), return true; otherwise return false.
        let rx = x.to_real(agent)?;
        let ry = y.to_real(agent)?;
        return Ok(rx == ry);
    }

    // 14. Return false.
    Ok(false)
}

/// [7.2.14 IsStrictlyEqual ( x, y )](https://tc39.es/ecma262/#sec-isstrictlyequal)
///
/// The abstract operation IsStrictlyEqual takes arguments x (an ECMAScript
/// language value) and y (an ECMAScript language value) and returns a Boolean.
/// It provides the semantics for the === operator.
pub(crate) fn is_strictly_equal(
    agent: &mut Agent,
    x: impl Into<Value> + Copy,
    y: impl Into<Value> + Copy,
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
        // a. Return Number::equal(x, y).
        return x.equal(agent, y);
    }

    // 3. Return SameValueNonNumber(x, y).
    same_value_non_number(agent, x, y)
}
