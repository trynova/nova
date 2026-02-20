// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [7.2 Testing and Comparison Operations](https://tc39.es/ecma262/#sec-testing-and-comparison-operations)

use crate::{
    ecmascript::{
        Agent, BigInt, ExceptionType, Function, InternalMethods, JsResult, NonRevokedProxy, Number,
        Numeric, Object, PreferredType, Primitive, String, Value, string_to_big_int,
        string_to_number, to_numeric_primitive, to_primitive, validate_non_revoked_proxy,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable},
    heap::{ArenaAccess, PrimitiveHeapAccess},
};

#[cfg(feature = "regexp")]
use crate::{
    ecmascript::{PropertyKey, get, to_boolean},
    heap::WellKnownSymbolIndexes,
};

/// ### [7.2.1 RequireObjectCoercible ( argument )](https://tc39.es/ecma262/#sec-requireobjectcoercible)
///
/// The abstract operation RequireObjectCoercible takes argument argument (an
/// ECMAScript language value) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. It throws an
/// error if argument is a value that cannot be converted to an Object using
/// ToObject. It is defined by [Table 14](https://tc39.es/ecma262/#table-requireobjectcoercible-results):
pub(crate) fn require_object_coercible<'gc>(
    agent: &mut Agent,
    argument: Value,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'static, Value<'static>> {
    if argument.is_undefined() || argument.is_null() {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Argument cannot be converted into an object",
            gc,
        ))
    } else {
        Ok(argument)
    }
}

/// ### [7.2.2 IsArray ( argument )](https://tc39.es/ecma262/#sec-isarray)
///
/// The abstract operation IsArray takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing a Boolean
/// or a throw completion.
pub(crate) fn is_array<'a, 'gc>(
    agent: &mut Agent,
    argument: impl Into<Value<'a>>,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    crate::engine::bind!(let argument = argument.into(), gc);

    match argument {
        // 1. If argument is not an Object, return false.
        // 2. If argument is an Array exotic object, return true.
        Value::Array(_) => Ok(true),
        // 3. If argument is a Proxy exotic object, then
        Value::Proxy(proxy) => {
            // a. Perform ? ValidateNonRevokedProxy(argument).
            // b. Let proxyTarget be argument.[[ProxyTarget]].
            let NonRevokedProxy { target, handler: _ } =
                validate_non_revoked_proxy(agent, proxy, gc)?;
            // c. Return ? IsArray(proxyTarget).
            is_array(agent, target, gc)
        }
        // 4. Return false.
        _ => Ok(false),
    }
}

/// ### [7.2.3 IsCallable ( argument )](https://tc39.es/ecma262/#sec-iscallable)
///
/// The abstract operation IsCallable takes argument argument (an ECMAScript
/// language value) and returns a Boolean. It determines if argument is a
/// callable function with a [[Call]] internal method.
///
/// > #### Note
/// > Nova breaks with the specification to narrow the types automatically, and
/// > returns an `Option<Function>`. Eventually this should become
/// > `Option<Callable>` once callable proxies are supported.
pub(crate) fn is_callable<'a, 'b>(
    argument: impl TryInto<Function<'b>>,
    _: NoGcScope<'a, '_>,
) -> Option<Function<'a>> {
    // 1. If argument is not an Object, return false.
    // 2. If argument has a [[Call]] internal method, return true.
    // 3. Return false.
    if let Ok(f) = argument.try_into() {
        Some(f)
    } else {
        // TODO: Once this handles callable Proxies, remove special case in
        // Proxy::is_callable.
        None
    }
}

/// ### [7.2.4 IsConstructor ( argument )](https://tc39.es/ecma262/#sec-isconstructor)
///
/// The abstract operation IsConstructor takes argument argument (an ECMAScript
/// language value) and returns a Boolean. It determines if argument is a
/// function object with a [[Construct]] internal method.
///
/// > #### Note
/// > Nova breaks with the specification to narrow the types automatically, and
/// > returns an `Option<Function>`. Eventually this should become
/// > `Option<Callable>` or `Option<Constructable>` once callable proxies are
/// > supported.
pub(crate) fn is_constructor<'a>(
    agent: &mut Agent,
    constructor: impl TryInto<Function<'a>>,
) -> Option<Function<'a>> {
    // 1. If argument is not an Object, return false.
    // TODO: Proxy
    let Ok(constructor) = constructor.try_into() else {
        return None;
    };
    // 2. If argument has a [[Construct]] internal method, return true.
    if constructor.is_constructor(agent) {
        Some(constructor)
    } else {
        // 3. Return false.
        None
    }
}

/// ### [7.2.6 IsRegExp ( argument )](https://tc39.es/ecma262/#sec-isregexp)
///
/// The abstract operation IsRegExp takes argument
/// argument (an ECMAScript language value) and returns either a normal completion containing a Boolean or a throw completion.
#[cfg(feature = "regexp")]
pub(crate) fn is_reg_exp<'a>(
    agent: &mut Agent,
    argument: Value,
    gc: GcScope,
) -> JsResult<'static, bool> {
    crate::engine::bind!(let argument = argument, gc);

    // 1. If argument is not an Object, return false.
    let Ok(argument) = Object::try_from(argument) else {
        return Ok(false);
    };

    let is_native_reg_exp = matches!(argument, Object::RegExp(_));

    // 2. Let matcher be ? Get(argument, %Symbol.match%).
    let matcher = get(
        agent,
        argument,
        PropertyKey::Symbol(WellKnownSymbolIndexes::Match.into()),
        gc,
    )?;

    // 3. If matcher is not undefined, return ToBoolean(matcher).
    if !matcher.is_undefined() {
        return Ok(to_boolean(agent, matcher));
    }

    // 4. If argument has a [[RegExpMatcher]] internal slot, return true.
    if is_native_reg_exp {
        return Ok(true);
    }

    // 5. Return false.
    Ok(false)
}
#[cfg(not(feature = "regexp"))]
#[inline(always)]
pub(crate) fn is_reg_exp<'a>(
    _agent: &mut Agent,
    _argument: Value,
    _gc: GcScope,
) -> JsResult<'static, bool> {
    Ok(false)
}

/// ### [7.2.5 IsExtensible ( O )](https://tc39.es/ecma262/#sec-isextensible-o)
///
/// The abstract operation IsExtensible takes argument O (an Object) and
/// returns either a normal completion containing a Boolean or a throw
/// completion. It is used to determine whether additional properties can be
/// added to O.
pub(crate) fn is_extensible<'a>(
    agent: &mut Agent,
    o: Object,
    gc: GcScope,
) -> JsResult<'static, bool> {
    // 1. Return ? O.[[IsExtensible]]().
    o.internal_is_extensible(agent, gc)
}

pub(crate) fn is_same_type<'a, V1: Copy + Into<Value<'a>>, V2: Copy + Into<Value<'a>>>(
    x: V1,
    y: V2,
) -> bool {
    (x.into().is_undefined() && y.into().is_undefined())
        || (x.into().is_null() && y.into().is_null())
        || (x.into().is_boolean() && y.into().is_boolean())
        || (x.into().is_string() && y.into().is_string())
        || (x.into().is_symbol() && y.into().is_symbol())
        || (x.into().is_number() && y.into().is_number())
        || (x.into().is_bigint() && y.into().is_bigint())
        || (x.into().is_object() && y.into().is_object())
}

/// ### [7.2.6 IsIntegralNumber ( argument )](https://tc39.es/ecma262/#sec-isintegralnumber)
pub(crate) fn is_integral_number<'a>(
    agent: &mut Agent,
    argument: impl Copy + Into<Value<'a>>,
) -> bool {
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
    if !argument.is_finite_(agent) {
        return false;
    }

    // 3. If truncate(ℝ(argument)) ≠ ℝ(argument), return false.
    // 4. Return true.
    // NOTE: Checking if the fractional component is 0.0 is the same as the
    // specification's operation.
    argument.to_real(agent).fract() == 0.0
}

/// ### [7.2.10 SameValue ( x, y )](https://tc39.es/ecma262/#sec-samevalue)
pub(crate) fn same_value<'a, V1: Copy + Into<Value<'a>>, V2: Copy + Into<Value<'a>>>(
    agent: &impl PrimitiveHeapAccess,
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
        return Number::same_value_(agent, x, y);
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
/// the difference between +0𝔽 and -0𝔽).
pub(crate) fn same_value_zero<'a>(
    agent: &impl PrimitiveHeapAccess,
    x: impl Copy + Into<Value<'a>>,
    y: impl Copy + Into<Value<'a>>,
) -> bool {
    let (x, y) = (Into::<Value>::into(x), Into::<Value>::into(y));

    // 1. If Type(x) is not Type(y), return false.
    if !is_same_type(x, y) {
        return false;
    }

    // 2. If x is a Number, then
    // NOTE: We need to convert both to a number because we use number
    // type-safety.
    if let (Ok(x), Ok(y)) = (Number::try_from(x), Number::try_from(y)) {
        // a. Return Number::sameValueZero(x, y).
        return Number::same_value_zero_(agent, x, y);
    }

    // 3. Return SameValueNonNumber(x, y).
    same_value_non_number(agent, x, y)
}

/// ### [7.2.12 SameValueNonNumber ( x, y )](https://tc39.es/ecma262/#sec-samevaluenonnumber)
pub(crate) fn same_value_non_number<'a, T: Copy + Into<Value<'a>>>(
    agent: &impl PrimitiveHeapAccess,
    x: T,
    y: T,
) -> bool {
    let x: Value = x.into();
    let y: Value = y.into();

    // 1. Assert: Type(x) is Type(y).
    debug_assert!(is_same_type(x, y));

    // 2. If x is either null or undefined, return true.
    if x.is_null() || x.is_undefined() {
        return true;
    }

    // 3. If x is a BigInt, then
    if let (Ok(x), Ok(y)) = (BigInt::try_from(x), BigInt::try_from(y)) {
        // a. Return BigInt::equal(x, y).
        return BigInt::equal(agent, x, y);
    }

    // 4. If x is a String, then
    if let (Ok(x), Ok(y)) = (String::try_from(x), String::try_from(y)) {
        // a. If x and y have the same length and the same code units in the same positions, return true; otherwise, return false.
        return String::eq_(agent, x, y);
    }

    // 5. If x is a Boolean, then
    if x.is_boolean() {
        // a. If x and y are both true or both false, return true; otherwise, return false.
        return x.is_true() == y.is_true();
    }

    // 6. NOTE: All other ECMAScript language values are compared by identity.
    // 7. If x is y, return true; otherwise, return false.
    x == y
}

/// ### [7.2.13 IsLessThan ( x, y, LeftFirst )](https://tc39.es/ecma262/#sec-islessthan)
///
/// The abstract operation IsLessThan takes arguments x (an ECMAScript language
/// value), y (an ECMAScript language value), and LeftFirst (a Boolean) and
/// returns either a normal completion containing either a Boolean or
/// undefined, or a throw completion. It provides the semantics for the
/// comparison x < y, returning true, false, or undefined (which indicates that
/// at least one operand is NaN). The LeftFirst flag is used to control the
/// order in which operations with potentially visible side-effects are
/// performed upon x and y. It is necessary because ECMAScript specifies left
/// to right evaluation of expressions. If LeftFirst is true, the x parameter
/// corresponds to an expression that occurs to the left of the y parameter's
/// corresponding expression. If LeftFirst is false, the reverse is the case
/// and operations must be performed upon y before x.
pub(crate) fn is_less_than<'a, const LEFT_FIRST: bool>(
    agent: &mut Agent,
    x: impl Into<Value<'a>> + Copy,
    y: impl Into<Value<'a>> + Copy,
    mut gc: GcScope,
) -> JsResult<'a, Option<bool>> {
    let (px, py, gc) = match (Primitive::try_from(x.into()), Primitive::try_from(y.into())) {
        (Ok(px), Ok(py)) => {
            let gc = gc.into_nogc();
            (px, py, gc)
        }
        (Ok(px), Err(_)) => {
            let px = px.scope(agent, gc.nogc());
            let py = to_primitive(agent, y.into(), Some(PreferredType::Number), gc.reborrow())?;
            let gc = gc.into_nogc();
            let px = px.get(agent).local();
            (px, py, gc)
        }
        (Err(_), Ok(py)) => {
            let py = py.scope(agent, gc.nogc());
            let px = to_primitive(agent, x.into(), Some(PreferredType::Number), gc.reborrow())?;
            let gc = gc.into_nogc();
            let py = py.get(agent).local();
            (px, py, gc)
        }
        (Err(_), Err(_)) => {
            if LEFT_FIRST {
                // 1. If LeftFirst is true, then
                // a. Let px be ? ToPrimitive(x, NUMBER).
                // b. Let py be ? ToPrimitive(y, NUMBER).
                let y: Value = y.into();
                let y = y.scope(agent, gc.nogc());
                let px = to_primitive(agent, x.into(), Some(PreferredType::Number), gc.reborrow())?
                    .scope(agent, gc.nogc());
                let py = to_primitive(
                    agent,
                    y.get(agent).local(),
                    Some(PreferredType::Number),
                    gc.reborrow(),
                )?;
                let gc = gc.into_nogc();
                let px = px.get(agent).local();
                (px, py, gc)
            } else {
                // 2. Else,
                // a. NOTE: The order of evaluation needs to be reversed to preserve left to right evaluation.
                // b. Let py be ? ToPrimitive(y, NUMBER).
                // c. Let px be ? ToPrimitive(x, NUMBER).
                let x: Value = x.into();
                let x = x.scope(agent, gc.nogc());
                let py = to_primitive(agent, y.into(), Some(PreferredType::Number), gc.reborrow())?
                    .scope(agent, gc.nogc());
                let px = to_primitive(
                    agent,
                    x.get(agent).local(),
                    Some(PreferredType::Number),
                    gc.reborrow(),
                )?;
                let gc = gc.into_nogc();
                let py = py.get(agent).local();
                (px, py, gc)
            }
        }
    };

    // 3. If px is a String and py is a String, then
    if px.is_string() && py.is_string() {
        // a. Let lx be the length of px.
        // b. Let ly be the length of py.
        // c. For each integer i such that 0 ≤ i < min(lx, ly), in ascending order, do
        // i. Let cx be the numeric value of the code unit at index i within px.
        // ii. Let cy be the numeric value of the code unit at index i within py.
        // iii. If cx < cy, return true.
        // iv. If cx > cy, return false.
        // d. If lx < ly, return true. Otherwise, return false.
        let sx = String::try_from(px).unwrap();
        let sy = String::try_from(py).unwrap();
        if let (Some(sx), Some(sy)) = (sx.as_str_(agent), sy.as_str_(agent)) {
            Ok(Some(sx < sy))
        } else {
            let lx = sx.utf16_len_(agent);
            let ly = sy.utf16_len_(agent);
            let l = lx.min(ly);
            let mut sx = sx.as_wtf8_(agent).to_ill_formed_utf16();
            let mut sy = sy.as_wtf8_(agent).to_ill_formed_utf16();
            for _ in 0..l {
                // i. Let cx be the numeric value of the code unit at index i within px.
                let cx = sx.next().unwrap();
                // ii. Let cy be the numeric value of the code unit at index i within py.
                let cy = sy.next().unwrap();
                // iii. If cx < cy, return true.
                match cx.cmp(&cy) {
                    std::cmp::Ordering::Less => {
                        return Ok(Some(true));
                    }
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Greater => {
                        return Ok(Some(false));
                    }
                }
                // iv. If cx > cy, return false.
            }
            // d. If lx < ly, return true. Otherwise, return false.
            Ok(Some(lx < ly))
        }
    }
    // 4. Else,
    else {
        // a. If px is a BigInt and py is a String, then
        if px.is_bigint() && py.is_string() {
            let Ok(px) = BigInt::try_from(px) else {
                unreachable!()
            };
            let Ok(py) = String::try_from(py) else {
                unreachable!()
            };

            // i. Let ny be StringToBigInt(py).
            crate::engine::bind!(let ny = string_to_big_int(agent, py, gc)?, gc);
            // ii. If ny is undefined, return undefined.
            // iii. Return BigInt::lessThan(px, ny).
            return Ok(Some(BigInt::less_than(agent, px, ny)));
        }

        // b. If px is a String and py is a BigInt, then
        if px.is_string() && py.is_bigint() {
            let Ok(px) = String::try_from(px) else {
                unreachable!()
            };
            let Ok(py) = BigInt::try_from(py) else {
                unreachable!()
            };

            // i. Let nx be StringToBigInt(px).
            crate::engine::bind!(let nx = string_to_big_int(agent, px, gc)?, gc);
            // ii. If nx is undefined, return undefined.
            // iii. Return BigInt::lessThan(nx, py).
            return Ok(Some(BigInt::less_than(agent, nx, py)));
        }

        // c. NOTE: Because px and py are primitive values, evaluation order is not important.
        // d. Let nx be ? ToNumeric(px).
        crate::engine::bind!(let nx = to_numeric_primitive(agent, px, gc)?, gc);

        // e. Let ny be ? ToNumeric(py).
        crate::engine::bind!(let ny = to_numeric_primitive(agent, py, gc)?, gc);

        // f. If Type(nx) is Type(ny), then
        if is_same_type(nx, ny) {
            // i. If nx is a Number, then
            if let Ok(nx) = Number::try_from(nx) {
                // 1. Return Number::lessThan(nx, ny).
                let ny = Number::try_from(ny).unwrap();
                return Ok(Number::less_than(agent, nx, ny));
            }
            // ii. Else,
            else {
                // 1. Assert: nx is a BigInt.
                let nx = BigInt::try_from(nx).unwrap();
                let ny = BigInt::try_from(ny).unwrap();

                // 2. Return BigInt::lessThan(nx, ny).
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
        Ok(Some(match (nx, ny) {
            (Numeric::Number(x), Numeric::Number(y)) => {
                x != y && x.get(agent).local() < y.get(agent).local()
            }
            (Numeric::Number(x), Numeric::Integer(y)) => {
                *x.get(agent).local() < y.into_i64() as f64
            }
            (Numeric::Number(x), Numeric::SmallF64(y)) => *x.get(agent).local() < y.into_f64(),
            (Numeric::Integer(x), Numeric::Number(y)) => {
                (x.into_i64() as f64) < *y.get(agent).local()
            }
            (Numeric::Integer(x), Numeric::Integer(y)) => x.into_i64() < y.into_i64(),
            (Numeric::Number(x), Numeric::BigInt(y)) => {
                y.get(agent).local().ge(x.get(agent).local())
            }
            (Numeric::Number(x), Numeric::SmallBigInt(y)) => {
                *x.get(agent).local() < y.into_i64() as f64
            }
            (Numeric::Integer(x), Numeric::SmallF64(y)) => (x.into_i64() as f64) < y.into_f64(),
            (Numeric::Integer(x), Numeric::BigInt(y)) => y.get(agent).local().ge(&x.into_i64()),
            (Numeric::Integer(x), Numeric::SmallBigInt(y)) => x.into_i64() < y.into_i64(),
            (Numeric::SmallF64(x), Numeric::Number(y)) => x.into_f64() < *y.get(agent).local(),
            (Numeric::SmallF64(x), Numeric::Integer(y)) => x.into_f64() < y.into_i64() as f64,
            (Numeric::SmallF64(x), Numeric::SmallF64(y)) => x.into_f64() < y.into_f64(),
            (Numeric::SmallF64(x), Numeric::BigInt(y)) => y.get(agent).local().ge(&x.into_f64()),
            (Numeric::SmallF64(x), Numeric::SmallBigInt(y)) => x.into_f64() < y.into_i64() as f64,
            (Numeric::BigInt(x), Numeric::Number(y)) => {
                x.get(agent).local().le(y.get(agent).local())
            }
            (Numeric::BigInt(x), Numeric::Integer(y)) => x.get(agent).local().le(&y.into_i64()),
            (Numeric::BigInt(x), Numeric::SmallF64(y)) => x.get(agent).local().le(&y.into_f64()),
            (Numeric::BigInt(x), Numeric::BigInt(y)) => {
                x.get(agent).local().data < y.get(agent).local().data
            }
            (Numeric::BigInt(x), Numeric::SmallBigInt(y)) => x.get(agent).local().le(&y.into_i64()),
            (Numeric::SmallBigInt(x), Numeric::Number(y)) => {
                (x.into_i64() as f64) < *y.get(agent).local()
            }
            (Numeric::SmallBigInt(x), Numeric::Integer(y)) => x.into_i64() < y.into_i64(),
            (Numeric::SmallBigInt(x), Numeric::SmallF64(y)) => (x.into_i64() as f64) < y.into_f64(),
            (Numeric::SmallBigInt(x), Numeric::BigInt(y)) => y.get(agent).local().ge(&x.into_i64()),
            (Numeric::SmallBigInt(x), Numeric::SmallBigInt(y)) => x.into_i64() < y.into_i64(),
        }))
    }
}

/// ### [7.2.14 IsLooselyEqual ( x, y )](https://tc39.es/ecma262/#sec-islooselyequal)
///
/// The abstract operation IsLooselyEqual takes arguments x (an ECMAScript
/// language value) and y (an ECMAScript language value) and returns either a
/// normal completion containing a Boolean or a throw completion. It provides
/// the semantics for the == operator.
pub(crate) fn is_loosely_equal<'a>(
    agent: &mut Agent,
    x: impl Into<Value<'a>> + Copy,
    y: impl Into<Value<'a>> + Copy,
    mut gc: GcScope,
) -> JsResult<'static, bool> {
    let x: Value = x.into();
    let y: Value = y.into();

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
    if let (Ok(x), Ok(y)) = (Number::try_from(x), String::try_from(y)) {
        // Note: We know statically that calling ToNumber on a string calls
        // StringToNumber, and IsLooselyEqual with two Numbers calls
        // IsStrictlyEqual, which calls Number::equal.
        let gc = gc.into_nogc();
        let y = string_to_number(agent, y, gc);
        return Ok(Number::equal_(agent, x, y));
    }

    // 6. If x is a String and y is a Number, return ! IsLooselyEqual(! ToNumber(x), y).
    if let (Ok(x), Ok(y)) = (String::try_from(x), Number::try_from(y)) {
        // Note: We know statically that calling ToNumber on a string calls
        // StringToNumber, and IsLooselyEqual with two Numbers calls
        // IsStrictlyEqual, which calls Number::equal.
        let gc = gc.into_nogc();
        let x = string_to_number(agent, x, gc);
        return Ok(Number::equal_(agent, x, y));
    }

    // 7. If x is a BigInt and y is a String, then
    if let (Ok(x), Ok(y)) = (BigInt::try_from(x), String::try_from(y)) {
        // a. Let n be StringToBigInt(y).
        // b. If n is undefined, return false.
        let gc = gc.into_nogc();
        if let Ok(n) = string_to_big_int(agent, y, gc) {
            // c. Return ! IsLooselyEqual(x, n).
            // Note: IsLooselyEqual with two BigInts calls IsStrictlyEqual
            // which eventually calls BigInt::euqla
            return Ok(BigInt::equal(agent, x, n));
        } else {
            return Ok(false);
        }
    }

    // 8. If x is a String and y is a BigInt, return ! IsLooselyEqual(y, x).
    if let (Ok(x), Ok(y)) = (String::try_from(x), BigInt::try_from(y)) {
        // Note: This flips the operands and re-enters the call.
        // We'll skip to the above punch-line with flipped parameters.

        // a. Let n be StringToBigInt(x).
        // b. If n is undefined, return false.
        let gc = gc.into_nogc();
        if let Ok(n) = string_to_big_int(agent, x, gc) {
            // c. Return ! IsLooselyEqual(x, n).
            // Note: IsLooselyEqual with two BigInts calls IsStrictlyEqual
            // which eventually calls BigInt::euqla
            return Ok(BigInt::equal(agent, y, n));
        } else {
            return Ok(false);
        }
    }

    // 9. If x is a Boolean, return ! IsLooselyEqual(! ToNumber(x), y).
    if let Ok(x) = bool::try_from(x) {
        let x = if x { 1 } else { 0 };
        return Ok(is_loosely_equal(agent, x, y, gc).unwrap());
    }

    // 10. If y is a Boolean, return ! IsLooselyEqual(x, ! ToNumber(y)).
    if let Ok(y) = bool::try_from(y) {
        let y = if y { 1 } else { 0 };
        return Ok(is_loosely_equal(agent, x, y, gc).unwrap());
    }

    // 11. If x is either a String, a Number, a BigInt, or a Symbol and y is an Object, return ! IsLooselyEqual(x, ? ToPrimitive(y)).
    if (x.is_string() || x.is_number() || x.is_bigint() || x.is_symbol()) && y.is_object() {
        let x = x.scope(agent, gc.nogc());
        let y = to_primitive(agent, y, None, gc.reborrow())?;
        return Ok(is_loosely_equal(agent, x.get(agent).local(), y, gc).unwrap());
    }

    // 12. If x is an Object and y is either a String, a Number, a BigInt, or a Symbol, return ! IsLooselyEqual(? ToPrimitive(x), y).
    if x.is_object() && (y.is_string() || y.is_number() || y.is_bigint() || y.is_symbol()) {
        let y = y.scope(agent, gc.nogc());
        let x = to_primitive(agent, x, None, gc.reborrow())?;
        return Ok(is_loosely_equal(agent, x, y.get(agent).local(), gc).unwrap());
    }

    // 13. If x is a BigInt and y is a Number, or if x is a Number and y is a BigInt, then
    if let Some((a, b)) = if let (Ok(x), Ok(y)) = (BigInt::try_from(x), Number::try_from(y)) {
        Some((x, y))
    } else if let (Ok(x), Ok(y)) = (Number::try_from(x), BigInt::try_from(y)) {
        Some((y, x))
    } else {
        None
    } {
        // a. If x is not finite or y is not finite, return false.
        // Note: BigInt is always finite.
        if !b.is_finite_(agent) {
            return Ok(false);
        }

        // b. If ℝ(x) = ℝ(y), return true; otherwise return false.
        let b = b.to_real(agent);

        // Compare BigInt with f64 using precise comparison.
        return Ok(match a {
            BigInt::BigInt(heap_big_int) => heap_big_int.get(agent).local() == &b,
            BigInt::SmallBigInt(small_big_int) => small_big_int.into_i64() as f64 == b,
        });
    }

    // 14. Return false.
    Ok(false)
}

/// ### [7.2.14 IsStrictlyEqual ( x, y )](https://tc39.es/ecma262/#sec-isstrictlyequal)
///
/// The abstract operation IsStrictlyEqual takes arguments x (an ECMAScript
/// language value) and y (an ECMAScript language value) and returns a Boolean.
/// It provides the semantics for the === operator.
pub(crate) fn is_strictly_equal<'a>(
    agent: &Agent,
    x: impl Into<Value<'a>> + Copy,
    y: impl Into<Value<'a>> + Copy,
) -> bool {
    let (x, y) = (x.into(), y.into());

    // 1. If Type(x) is not Type(y), return false.
    if !is_same_type(x, y) {
        return false;
    }

    // 2. If x is a Number, then
    // NOTE: We need to convert both to a number because we use number
    // type-safety.
    if let (Ok(x), Ok(y)) = (Number::try_from(x), Number::try_from(y)) {
        // a. Return Number::equal(x, y).
        return Number::equal_(agent, x, y);
    }

    // 3. Return SameValueNonNumber(x, y).
    same_value_non_number(agent, x, y)
}
