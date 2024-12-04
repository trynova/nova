// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::f64::consts;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_number, to_number_primitive, to_uint32},
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, Primitive, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::context::{GcScope, NoGcScope},
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct MathObject;

struct MathObjectAbs;
impl Builtin for MathObjectAbs {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.abs;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::abs);
}

struct MathObjectAcos;
impl Builtin for MathObjectAcos {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.acos;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::acos);
}
struct MathObjectAcosh;
impl Builtin for MathObjectAcosh {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.acosh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::acosh);
}
struct MathObjectAsin;
impl Builtin for MathObjectAsin {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.asin;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::asin);
}
struct MathObjectAsinh;
impl Builtin for MathObjectAsinh {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.asinh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::asinh);
}
struct MathObjectAtan;
impl Builtin for MathObjectAtan {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.atan;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::atan);
}
struct MathObjectAtanh;
impl Builtin for MathObjectAtanh {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.atanh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::atanh);
}
struct MathObjectAtan2;
impl Builtin for MathObjectAtan2 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.atan2;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::atan2);
}
struct MathObjectCbrt;
impl Builtin for MathObjectCbrt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.cbrt;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::cbrt);
}
struct MathObjectCeil;
impl Builtin for MathObjectCeil {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.ceil;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::ceil);
}
struct MathObjectClz32;
impl Builtin for MathObjectClz32 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.clz32;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::clz32);
}
struct MathObjectCos;
impl Builtin for MathObjectCos {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.cos;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::cos);
}
struct MathObjectCosh;
impl Builtin for MathObjectCosh {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.cosh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::cosh);
}
struct MathObjectExp;
impl Builtin for MathObjectExp {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.exp;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::exp);
}
struct MathObjectExpm1;
impl Builtin for MathObjectExpm1 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.expm1;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::expm1);
}
struct MathObjectFloor;
impl Builtin for MathObjectFloor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.floor;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::floor);
}
struct MathObjectFround;
impl Builtin for MathObjectFround {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fround;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::fround);
}
struct MathObjectHypot;
impl Builtin for MathObjectHypot {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.hypot;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::hypot);
}
struct MathObjectImul;
impl Builtin for MathObjectImul {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.imul;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::imul);
}
struct MathObjectLog;
impl Builtin for MathObjectLog {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.log;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log);
}
struct MathObjectLog1p;
impl Builtin for MathObjectLog1p {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.log1p;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log1p);
}
struct MathObjectLog10;
impl Builtin for MathObjectLog10 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.log10;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log10);
}
struct MathObjectLog2;
impl Builtin for MathObjectLog2 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.log2;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log2);
}
struct MathObjectMax;
impl Builtin for MathObjectMax {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.max;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::max);
}
struct MathObjectMin;
impl Builtin for MathObjectMin {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.min;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::min);
}
struct MathObjectPow;
impl Builtin for MathObjectPow {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.pow;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::pow);
}
struct MathObjectRandom;
impl Builtin for MathObjectRandom {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.random;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::random);
}
struct MathObjectRound;
impl Builtin for MathObjectRound {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.round;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::round);
}
struct MathObjectSign;
impl Builtin for MathObjectSign {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sign;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sign);
}
struct MathObjectSin;
impl Builtin for MathObjectSin {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sin;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sin);
}
struct MathObjectSinh;
impl Builtin for MathObjectSinh {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sinh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sinh);
}
struct MathObjectSqrt;
impl Builtin for MathObjectSqrt {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sqrt;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sqrt);
}
struct MathObjectTan;
impl Builtin for MathObjectTan {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.tan;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::tan);
}
struct MathObjectTanh;
impl Builtin for MathObjectTanh {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.tanh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::tanh);
}
struct MathObjectTrunc;
impl Builtin for MathObjectTrunc {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.trunc;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::trunc);
}

impl MathObject {
    fn abs(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;
        Ok(n.abs(agent).into_value())
    }

    fn acos(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?.into_f64(agent);
        // 2. If n is NaN, n > 1𝔽, or n < -1𝔽, return NaN.
        // 3. If n is 1𝔽, return +0𝔽.
        // 4. Return an implementation-approximated Number value representing the result of the inverse cosine of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.acos()))
    }

    fn acosh(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        // 2. If n is either NaN or +∞𝔽, return n.
        if n.is_nan(agent) || n.is_pos_infinity(agent) {
            return Ok(n.into_value());
        }

        // 3. If n is 1𝔽, return +0𝔽.
        if n.is_pos_one(agent) {
            return Ok(Value::pos_zero());
        }

        let n = n.into_f64(agent);

        // 4. If n < 1𝔽, return NaN.
        // 5. Return an implementation-approximated Number value representing the result of the inverse hyperbolic cosine of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.acosh()))
    }

    fn asin(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n.
        if n.is_nan(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }

        let n = n.into_f64(agent);

        // 3. If n > 1𝔽 or n < -1𝔽, return NaN.
        if !(-1.0..=1.0).contains(&n) {
            return Ok(Value::nan());
        }

        // 4. Return an implementation-approximated Number value representing the result of the inverse sine of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.asin()))
    }

    fn asinh(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        // 2. If n is not finite or n is either +0𝔽 or -0𝔽, return n.
        if !n.is_finite(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }

        // 3. Return an implementation-approximated Number value representing the result of the inverse hyperbolic sine of ℝ(n).
        let result = n.into_f64(agent).asinh();
        Ok(Value::from_f64(agent, gc.into_nogc(), result))
    }

    fn atan(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n.
        if n.is_nan(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }

        // 3. If n is +∞𝔽, return an implementation-approximated Number value representing π / 2.
        if n.is_pos_infinity(agent) {
            return Ok(Value::from_f64(agent, gc.into_nogc(), consts::FRAC_PI_2));
        }

        // 4. If n is -∞𝔽, return an implementation-approximated Number value representing -π / 2.
        if n.is_neg_infinity(agent) {
            return Ok(Value::from_f64(agent, gc.into_nogc(), -consts::FRAC_PI_2));
        }

        // 5. Return an implementation-approximated Number value representing the result of the inverse tangent of ℝ(n).
        let result = n.into_f64(agent).atan();
        Ok(Value::from_f64(agent, gc.into_nogc(), result))
    }

    fn atanh(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n.
        if n.is_nan(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }

        // 4. If n is 1𝔽, return +∞𝔽.
        if n.is_pos_one(agent) {
            return Ok(Value::pos_inf());
        }

        // 5. If n is -1𝔽, return -∞𝔽.
        if n.is_neg_one(agent) {
            return Ok(Value::neg_inf());
        }

        let n = n.into_f64(agent);

        // 3. If n > 1𝔽 or n < -1𝔽, return NaN.
        if !(-1.0..=1.0).contains(&n) {
            return Ok(Value::nan());
        }

        // 6. Return an implementation-approximated Number value representing the result of the inverse hyperbolic tangent of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.atanh()))
    }

    fn atan2(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let ny be ? ToNumber(y).
        let ny = to_number(agent, gc.reborrow(), arguments.get(0))?.into_f64(agent);
        // 2. Let nx be ? ToNumber(x).
        let nx = to_number(agent, gc.reborrow(), arguments.get(1))?.into_f64(agent);

        // 3. If ny is NaN or nx is NaN, return NaN.
        if ny.is_nan() || nx.is_nan() {
            return Ok(Value::nan());
        }

        // 4. If ny is +∞𝔽, then
        if ny.is_infinite() && ny.is_sign_positive() {
            if nx.is_infinite() {
                // a. If nx is +∞𝔽, return an implementation-approximated Number value representing π / 4.
                if nx.is_sign_positive() {
                    return Ok(Value::from_f64(agent, gc.into_nogc(), consts::FRAC_PI_4));
                }
                // b. If nx is -∞𝔽, return an implementation-approximated Number value representing 3π / 4.
                if nx.is_sign_negative() {
                    return Ok(Value::from_f64(
                        agent,
                        gc.into_nogc(),
                        3.0 * consts::FRAC_PI_4,
                    ));
                }
            }
            // c. Return an implementation-approximated Number value representing π / 2.
            return Ok(Value::from_f64(agent, gc.into_nogc(), consts::FRAC_PI_2));
        }

        // 5. If ny is -∞𝔽, then
        if ny.is_infinite() && ny.is_sign_negative() {
            if nx.is_infinite() {
                // a. If nx is +∞𝔽, return an implementation-approximated Number value representing -π / 4.
                if nx.is_sign_positive() {
                    return Ok(Value::from_f64(agent, gc.into_nogc(), -consts::FRAC_PI_4));
                }
                // b. If nx is -∞𝔽, return an implementation-approximated Number value representing -3π / 4.
                if nx.is_sign_negative() {
                    return Ok(Value::from_f64(
                        agent,
                        gc.into_nogc(),
                        -3.0 * consts::FRAC_PI_4,
                    ));
                }
            }
            // c. Return an implementation-approximated Number value representing -π / 2.
            return Ok(Value::from_f64(agent, gc.into_nogc(), -consts::FRAC_PI_2));
        }

        // 6. If ny is +0𝔽, then
        if ny == 0.0 {
            if ny.is_sign_positive() {
                // a. If nx > +0𝔽 or nx is +0𝔽, return +0𝔽.
                if nx >= 0.0 && nx.is_sign_positive() {
                    return Ok(Value::pos_zero());
                }
                // b. Return an implementation-approximated Number value representing π.
                return Ok(Value::from_f64(agent, gc.into_nogc(), consts::PI));
            }

            // 7. If ny is -0𝔽, then
            if ny.is_sign_negative() {
                // a. If nx > +0𝔽 or nx is +0𝔽, return -0𝔽.
                if nx >= 0.0 && nx.is_sign_positive() {
                    return Ok(Value::neg_zero());
                }
                // b. Return an implementation-approximated Number value representing -π.
                return Ok(Value::from_f64(agent, gc.into_nogc(), -consts::PI));
            }
        }

        // 8. Assert: ny is finite and is neither +0𝔽 nor -0𝔽.
        assert!(ny.is_finite() && ny != 0.0);

        // 9. If ny > +0𝔽, then
        if ny > 0.0 {
            if nx.is_infinite() {
                // a. If nx is +∞𝔽, return +0𝔽.
                if nx.is_sign_positive() {
                    return Ok(Value::pos_zero());
                }
                // b. If nx is -∞𝔽, return an implementation-approximated Number value representing π.
                if nx.is_sign_negative() {
                    return Ok(Value::from_f64(agent, gc.into_nogc(), consts::PI));
                }
            }
            // c. If nx is either +0𝔽 or -0𝔽, return an implementation-approximated Number value representing π / 2.
            if nx == 0.0 {
                return Ok(Value::from_f64(agent, gc.into_nogc(), consts::FRAC_PI_2));
            }
        }

        // 10. If ny < -0𝔽, then
        if ny < 0.0 {
            // a. If nx is +∞𝔽, return -0𝔽.
            if nx.is_sign_positive() {
                return Ok(Value::neg_zero());
            }
            // b. If nx is -∞𝔽, return an implementation-approximated Number value representing -π.
            if nx.is_sign_negative() {
                return Ok(Value::from_f64(agent, gc.into_nogc(), -consts::PI));
            }
            // c. If nx is either +0𝔽 or -0𝔽, return an implementation-approximated Number value representing -π / 2.
            if nx == 0.0 {
                return Ok(Value::from_f64(agent, gc.into_nogc(), -consts::FRAC_PI_2));
            }
        }

        // 11. Assert: nx is finite and is neither +0𝔽 nor -0𝔽.
        assert!(nx.is_finite() && nx != 0.0);

        // 12. Let r be the inverse tangent of abs(ℝ(ny) / ℝ(nx)).
        let mut r = (ny / nx).atan();

        // 13. If nx < -0𝔽, then
        if nx < 0.0 {
            // a. If ny > +0𝔽, set r to π - r.
            if ny > 0.0 {
                r = consts::PI - r;
            } else {
                // b. Else, set r to -π + r.
                r += -consts::PI;
            }
        }
        // 14. Else,
        else {
            // a. If ny < -0𝔽, set r to -r.
            if ny < 0.0 {
                r = -r;
            }
        }

        // 15. Return an implementation-approximated Number value representing r.
        Ok(Value::from_f64(agent, gc.into_nogc(), r))
    }

    fn cbrt(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?.unbind();
        let gc = gc.into_nogc();
        let n = n.bind(gc);

        // 2. If n is not finite or n is either +0𝔽 or -0𝔽, return n.
        if !n.is_finite(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }

        // 3. Return an implementation-approximated Number value representing the result of the cube root of ℝ(n).
        Ok(Value::from_f64(agent, gc, n.into_f64(agent).cbrt()))
    }

    fn ceil(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let n = arguments.get(0);
        // 4. If n is an integral Number, return n.
        if n.is_integer() {
            return Ok(n);
        }

        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), n)?;

        // 4. If n is an integral Number, return n.
        if n.is_integer(agent) {
            return Ok(n.into_value());
        }

        // 2. If n is not finite or n is either +0𝔽 or -0𝔽, return n.
        if !n.is_finite(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }

        let n = n.into_f64(agent);

        // 3. If n < -0𝔽 and n > -1𝔽, return -0𝔽.
        if n < -0.0 && n > -1.0 {
            return Ok(Value::neg_zero());
        }

        // 5. Return the smallest (closest to -∞) integral Number value that is not less than n.
        Ok(Value::from_f64(agent, gc.into_nogc(), n.ceil()))
    }

    fn clz32(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToUint32(x).
        let n = to_uint32(agent, gc.reborrow(), arguments.get(0))?;

        // 2. Let p be the number of leading zero bits in the unsigned 32-bit binary representation of n.
        let p = n.leading_zeros();

        // 3. Return 𝔽(p).
        Ok(Value::from(p))
    }

    fn cos(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?.into_f64(agent);

        // 2. If n is not finite, return NaN.
        if !n.is_finite() {
            return Ok(Value::nan());
        }

        // 3. If n is either +0𝔽 or -0𝔽, return 1𝔽.
        if n == 0.0 {
            return Ok(Value::from(1));
        }

        // 4. Return an implementation-approximated Number value representing the result of the cosine of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.cos()))
    }

    fn cosh(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?.into_f64(agent);

        // 2. If n is NaN, return NaN.
        if n.is_nan() {
            return Ok(Value::nan());
        }

        // 3. If n is either +∞𝔽 or -∞𝔽, return +∞𝔽.
        if n.is_infinite() {
            return Ok(Number::pos_inf().into_value());
        }

        // 4. If n is either +0𝔽 or -0𝔽, return 1𝔽.
        if n == 0.0 {
            return Ok(Value::from(1));
        }

        // 5. Return an implementation-approximated Number value representing the result of the hyperbolic cosine of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.cosh()))
    }

    fn exp(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        //1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        //2. If n is either NaN or +∞𝔽, return n.
        if n.is_nan(agent) || n.is_pos_infinity(agent) {
            return Ok(n.into_value());
        }

        let n = n.into_f64(agent);

        //3. If n is either +0𝔽 or -0𝔽, return 1𝔽.
        if n == 0.0 {
            return Ok(Value::from(1));
        }

        //4. If n is -∞𝔽, return +0𝔽.
        if n.is_infinite() {
            return Ok(Value::pos_zero());
        }

        //5. Return an implementation-approximated Number value representing the result of the exponential function of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.exp()))
    }

    fn expm1(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        // 2. If n is one of NaN, +0𝔽, -0𝔽, or +∞𝔽, return n.
        if n.is_nan(agent)
            || n.is_pos_zero(agent)
            || n.is_neg_zero(agent)
            || n.is_pos_infinity(agent)
        {
            return Ok(n.into_value());
        }

        let n = n.into_f64(agent);

        // 3. If n is -∞𝔽, return -1𝔽.
        if n.is_infinite() {
            return Ok(Value::from(-1));
        }

        // 4. Return an implementation-approximated Number value representing the result of subtracting 1 from the exponential function of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.exp_m1()))
    }

    fn floor(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let n = arguments.get(0);

        // 4. If n is an integral Number, return n.
        if n.is_integer() {
            return Ok(n.into_value());
        }

        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), n)?;

        // 4. If n is an integral Number, return n.
        if n.is_integer(agent) {
            return Ok(n.into_value());
        }

        // 2. If n is not finite or n is either +0𝔽 or -0𝔽, return n.
        if !n.is_finite(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }

        let n = n.into_f64(agent);

        // 3. If n < 1𝔽 and n > +0𝔽, return +0𝔽.
        if n < 1.0 && n > 0.0 {
            return Ok(Value::pos_zero());
        }

        // 5. Return the greatest (closest to +∞) integral Number value that is not greater than n.
        Ok(Value::from_f64(agent, gc.into_nogc(), n.floor()))
    }

    fn fround(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;

        // 2. If n is NaN, return NaN.
        if n.is_nan(agent) {
            return Ok(Value::nan());
        }

        // 3. If n is one of +0𝔽, -0𝔽, +∞𝔽, or -∞𝔽, return n.
        if n.is_pos_zero(agent)
            || n.is_neg_zero(agent)
            || n.is_pos_infinity(agent)
            || n.is_neg_infinity(agent)
        {
            return Ok(n.into_value());
        }

        // 4. Let n32 be the result of converting n to IEEE 754-2019 binary32 format using roundTiesToEven mode.
        let n32 = n.into_f32(agent);

        // 5. Let n64 be the result of converting n32 to IEEE 754-2019 binary64 format.
        let n64 = n32 as f64;

        // 6. Return the ECMAScript Number value corresponding to n64.
        Ok(Value::from_f64(agent, gc.into_nogc(), n64))
    }

    fn hypot(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let coerced be a new empty List.

        // 2. For each element arg of args, do
        // 4. Let onlyZero be true.
        let mut sum_of_squares = 0.0;
        let mut only_zero = true;
        let mut contains_infinity = false;
        let mut contains_nan = false;
        for &arg in arguments.iter() {
            // a. Let n be ? ToNumber(arg).
            let n = to_number(agent, gc.reborrow(), arg)?.into_f64(agent);

            // 3. For each element number of coerced, do
            if n.is_infinite() {
                // a. If number is either +∞𝔽 or -∞𝔽, return +∞𝔽.
                contains_infinity = true;
            } else if n.is_nan() {
                // a. If number is NaN, return NaN.
                contains_nan = true;
            } else if n != 0.0 {
                // b. If number is neither +0𝔽 nor -0𝔽, set onlyZero to false.
                only_zero = false;
                // b. Append n to coerced.
                sum_of_squares += n * n;
            }
        }

        // 3. For each element number of coerced, do
        // a. If number is either +∞𝔽 or -∞𝔽, return +∞𝔽.
        if contains_infinity {
            return Ok(Value::pos_inf());
        }

        // 5. For each element number of coerced, do
        if contains_nan {
            // a. If number is NaN, return NaN.
            return Ok(Value::nan());
        }

        // 6. If onlyZero is true, return +0𝔽.
        if only_zero {
            return Ok(Value::pos_zero());
        }

        // 7. Return an implementation-approximated Number value representing the square root of the sum of squares of the mathematical values of the elements of coerced.
        return Ok(Value::from_f64(
            agent,
            gc.into_nogc(),
            sum_of_squares.sqrt(),
        ));
    }

    fn imul(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let a be ℝ(? ToUint32(x)).
        let a = to_uint32(agent, gc.reborrow(), arguments.get(0))?;

        // 2. Let b be ℝ(? ToUint32(y)).
        let b = to_uint32(agent, gc.reborrow(), arguments.get(1))?;

        // 3. Let product be (a × b) modulo 2**32.
        let product = a.wrapping_mul(b);

        // 4. If product ≥ 2**31, return 𝔽(product - 2**32); otherwise return 𝔽(product).
        Ok(Value::from(product as i32))
    }

    fn log(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);

        // 2. If n is either NaN or +∞𝔽, return n.
        if n.is_nan() || n == f64::INFINITY {
            return Ok(n_number.into_value());
        }

        // 3. If n is 1𝔽, return +0𝔽.
        if n == 1.0 {
            return Ok(Value::pos_zero());
        }

        // 4. If n is either +0𝔽 or -0𝔽, return -∞𝔽.
        if n == 0.0 {
            return Ok(Value::neg_inf());
        }

        // 5. If n < -0𝔽, return NaN.
        if n.is_sign_negative() {
            return Ok(Value::nan());
        }

        // 6. Return an implementation-approximated Number value representing the result of the natural logarithm of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.ln()))
    }

    fn log1p(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is one of NaN, +0𝔽, -0𝔽, or +∞𝔽, return n.
        if n.is_nan() || n == 0.0 || n == f64::INFINITY {
            return Ok(n_number.into_value());
        }
        // 3. If n is -1𝔽, return -∞𝔽.
        if n == -1.0 {
            return Ok(Value::neg_inf());
        }
        // 4. If n < -1𝔽, return NaN.
        if n < -1.0 {
            return Ok(Value::nan());
        }
        // 5. Return an implementation-approximated Number value representing the natural logarithm of 1 + ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.ln_1p()))
    }

    fn log10(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is either NaN or +∞𝔽, return n.
        if n.is_nan() || n == f64::INFINITY {
            return Ok(n_number.into_value());
        }
        // 3. If n is 1𝔽, return +0𝔽.
        if n == 1.0 {
            return Ok(Value::pos_zero());
        }
        // 4. If n is either +0𝔽 or -0𝔽, return -∞𝔽.
        if n == 0.0 {
            return Ok(Value::neg_inf());
        }
        // 5. If n < -0𝔽, return NaN.
        if n < -0.0 {
            return Ok(Value::nan());
        }

        // 6. Return an implementation-approximated Number value representing the base 10 logarithm of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.log10()))
    }

    fn log2(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is either NaN or +∞𝔽, return n.
        if n.is_nan() || n == f64::INFINITY {
            return Ok(n_number.into_value());
        }
        // 3. If n is 1𝔽, return +0𝔽.
        if n == 1.0 {
            return Ok(Value::pos_zero());
        }
        // 4. If n is either +0𝔽 or -0𝔽, return -∞𝔽.
        if n == 0.0 {
            return Ok(Value::neg_inf());
        }
        // 5. If n < -0𝔽, return NaN.
        if n < -0.0 {
            return Ok(Value::nan());
        }
        // 6. Return an implementation-approximated Number value
        Ok(Value::from_f64(agent, gc.into_nogc(), n.log2()))
    }

    fn max(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        if arguments.is_empty() {
            return Ok(Value::neg_inf());
        }
        // 1. Let coerced be a new empty List.
        // Note: We avoid keeping a list by doing a reduce-like strategy.

        let mut only_ints = true;
        let mut contained_nan = false;

        // 3. Let highest be -∞𝔽.
        let mut highest_i64 = i64::MIN;
        let mut highest_f64 = f64::NEG_INFINITY;

        // 2. For each element arg of args, do
        let mut i = None;
        // We hope that this loop is all we need: It will never perform GC and
        // we avoid the need to root our arguments to this call scope. Rooting
        // them would require a heap allocation.
        for (idx, &arg) in arguments.iter().enumerate() {
            let n = if let Ok(n) = Number::try_from(arg) {
                Some(n)
            } else if let Ok(prim) = Primitive::try_from(arg) {
                Some(to_number_primitive(agent, gc.nogc(), prim)?)
            } else {
                None
            };
            // 4. For each element number of coerced, do
            if let Some(number) = n {
                if let Number::Integer(int) = number {
                    // c. If n > highest, set highest to n.
                    highest_i64 = highest_i64.max(int.into_i64());
                } else {
                    only_ints = false;

                    let value = number.into_f64(agent);
                    if value.is_nan() {
                        contained_nan = true;
                    } else {
                        // b. If n is +0𝔽 and highest is -0𝔽, set highest to +0𝔽.
                        // Note: This is handled automatically as +0 is integer.
                        // c. If n > highest, set highest to n.
                        highest_f64 = highest_f64.max(value);
                    }
                }
            } else {
                // Non-primitive argument encountered.
                i = Some(idx);
                break;
            }
        }

        if let Some(i) = i {
            // Note: We encountered non-primitive values. We're possibly
            // calling into user-provided JavaScript and triggering GC.

            let slow_nan = max_slow_path(
                agent,
                gc.reborrow(),
                &mut only_ints,
                &mut highest_i64,
                &mut highest_f64,
                &arguments[i..],
            )?;
            if slow_nan {
                contained_nan = true;
            }
        }

        // a. If number is NaN, return NaN.
        if contained_nan {
            return Ok(Value::nan());
        }

        // 5. Return highest.
        if only_ints {
            // SAFETY: Because we know that we only got safe integers, we
            // know that the maximum integer is also a safe integer.
            Ok(Number::try_from(highest_i64).unwrap().into_value())
        } else {
            // Note: This is potentially one unnecessary heap f64 allocation.
            // We may have got the maximum f64 from the heap and now we push it
            // back there without reusing the original Number. This just makes
            // the code simpler.
            if highest_i64 != i64::MIN {
                // b. If n is +0𝔽 and highest is -0𝔽, set highest to +0𝔽.
                // Note: This happens automatically as +0.max(-0) is +0.
                highest_f64 = (highest_i64 as f64).max(highest_f64);
            }
            let result = Number::from_f64(agent, gc.nogc(), highest_f64);
            Ok(result.into_value())
        }
    }

    fn min(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        if arguments.is_empty() {
            return Ok(Value::pos_inf());
        }

        // 1. Let coerced be a new empty List.
        // Note: We avoid keeping a list by doing a reduce-like strategy.

        let mut only_ints = true;
        let mut contained_nan = false;

        // 3. Let lowest be +∞𝔽.
        let mut lowest_i64 = i64::MAX;
        let mut lowest_f64 = f64::INFINITY;

        // 2. For each element arg of args, do
        let mut i = None;
        // We hope that this loop is all we need: It will never perform GC and
        // we avoid the need to root our arguments to this call scope. Rooting
        // them would require a heap allocation.
        for (idx, &arg) in arguments.iter().enumerate() {
            let n = if let Ok(n) = Number::try_from(arg) {
                Some(n)
            } else if let Ok(prim) = Primitive::try_from(arg) {
                Some(to_number_primitive(agent, gc.nogc(), prim)?)
            } else {
                None
            };
            // 4. For each element number of coerced, do
            if let Some(number) = n {
                if let Number::Integer(int) = number {
                    // c. If number < lowest, set lowest to number.
                    lowest_i64 = lowest_i64.min(int.into_i64());
                } else {
                    only_ints = false;

                    let number = number.into_f64(agent);
                    // a. If number is NaN, return NaN.
                    if number.is_nan() {
                        contained_nan = true;
                    } else {
                        // b. If number is -0𝔽 and lowest is +0𝔽, set lowest to -0𝔽.
                        // Note: We'll handle this later. +0 is always an integer.
                        // c. If number < lowest, set lowest to number.
                        lowest_f64 = lowest_f64.min(number);
                    }
                }
            } else {
                // Non-primitive argument encountered.
                i = Some(idx);
                break;
            }
        }

        if let Some(i) = i {
            // Note: We encountered non-primitive values. We're possibly
            // calling into user-provided JavaScript and triggering GC.

            let slow_nan = min_slow_path(
                agent,
                gc.reborrow(),
                &mut only_ints,
                &mut lowest_i64,
                &mut lowest_f64,
                &arguments[i..],
            )?;
            if slow_nan {
                contained_nan = true;
            }
        }

        // a. If number is NaN, return NaN.
        if contained_nan {
            return Ok(Value::nan());
        }

        // 5. Return lowest.
        if only_ints {
            // SAFETY: Because we know that we only got safe integers, we
            // know that the maximum integer is also a safe integer.
            Ok(Number::try_from(lowest_i64).unwrap().into_value())
        } else {
            // Note: This is potentially one unnecessary heap f64 allocation.
            // We may have got the minimum f64 from the heap and now we push it
            // back there without reusing the original Number. This just makes
            // the code simpler.
            if lowest_i64 != i64::MAX {
                // b. If number is -0𝔽 and lowest is +0𝔽, set lowest to -0𝔽.
                if lowest_f64 != -0.0 || !lowest_f64.is_sign_negative() || lowest_i64 != 0 {
                    lowest_f64 = lowest_f64.min(lowest_i64 as f64);
                }
            }
            Ok(Number::from_f64(agent, gc.nogc(), lowest_f64).into_value())
        }
    }

    fn pow(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let base = arguments.get(0);
        let exponent = arguments.get(1);
        let (base, exponent) = if let (Ok(base), Ok(exponent)) =
            (Number::try_from(base), Number::try_from(exponent))
        {
            (base.bind(gc.nogc()), exponent.bind(gc.nogc()))
        } else if let (Ok(base), Ok(exponent)) =
            (Primitive::try_from(base), Primitive::try_from(exponent))
        {
            let base = to_number_primitive(agent, gc.nogc(), base)?;
            let exponent = to_number_primitive(agent, gc.nogc(), exponent)?;
            (base.bind(gc.nogc()), exponent.bind(gc.nogc()))
        } else {
            let exponent = exponent.scope(agent, gc.nogc());
            let base = to_number(agent, gc.reborrow(), base)?
                .unbind()
                .scope(agent, gc.nogc());
            let exponent = to_number(agent, gc.reborrow(), exponent.get(agent))?.unbind();
            (base.get(agent).bind(gc.nogc()), exponent.bind(gc.nogc()))
        };
        let gc = gc.nogc();
        if let (Number::Integer(base), Number::Integer(exponent)) = (base, exponent) {
            let base = base.into_i64();
            let exponent = exponent.into_i64();
            if let Ok(exponent) = u32::try_from(exponent) {
                if let Some(result) = base.checked_pow(exponent) {
                    if let Ok(result) = Number::try_from(result) {
                        return Ok(result.into_value());
                    } else {
                        return Ok(Value::from_f64(agent, gc, result as f64));
                    }
                } else if let Some(result) = (base as i128).checked_pow(exponent) {
                    return Ok(Value::from_f64(agent, gc, result as f64));
                } else {
                    return Ok(Value::from_f64(
                        agent,
                        gc,
                        (base as f64).powf(exponent as f64),
                    ));
                }
            } else if let Ok(exponent) = i32::try_from(exponent) {
                let result = (base as f64).powi(exponent);
                return Ok(Value::from_f64(agent, gc, result));
            } else {
                let result = (base as f64).powf(exponent as f64);
                return Ok(Value::from_f64(agent, gc, result));
            }
        }
        Ok(Number::exponentiate(agent, base, exponent).into_value())
    }

    fn random(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(Value::from_f64(
            agent,
            gc.into_nogc(),
            rand::random::<f64>(),
        ))
    }

    fn round(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let n = arguments.get(0);
        if n.is_integer() {
            return Ok(n);
        }
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), n)?;

        // 2. If n is not finite or n is an integral Number, return n.
        if !n.is_finite(agent) || n.is_integer(agent) {
            return Ok(n.into_value());
        }

        let n = n.into_f64(agent);

        // 3. If n < 0.5𝔽 and n > +0𝔽, return +0𝔽.
        if n < 0.5 && n > 0.0 {
            return Ok(Value::pos_zero());
        }

        // 4. If n < -0𝔽 and n ≥ -0.5𝔽, return -0𝔽.
        if (-0.5..-0.0).contains(&n) {
            return Ok(Value::neg_zero());
        }

        // 5. Return the integral Number closest to n, preferring the Number closer to +∞ in the case of a tie.
        Ok(Value::from_f64(agent, gc.into_nogc(), n.round()))
    }

    fn sign(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n = to_number(agent, gc.reborrow(), arguments.get(0))?;
        // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n.
        if n.is_nan(agent) || n.is_pos_zero(agent) || n.is_neg_zero(agent) {
            return Ok(n.into_value());
        }
        // 3. If n < -0𝔽, return -1𝔽.
        if n.is_sign_negative(agent) {
            return Ok(Value::from(-1));
        }
        // 4. Return 1𝔽.
        Ok(Value::from(1))
    }

    fn sin(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n.
        if n.is_nan() || n == 0.0 {
            return Ok(n_number.into_value());
        }
        // 3. If n is either +∞𝔽 or -∞𝔽, return NaN.
        if n.is_infinite() {
            return Ok(Value::nan());
        }
        // 4. Return an implementation-approximated Number value representing the sine of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.sin()))
    }

    fn sinh(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is not finite or n is either +0𝔽 or -0𝔽, return n.
        if !n.is_finite() || n == 0.0 {
            return Ok(n_number.into_value());
        }
        // 3. Return an implementation-approximated Number value representing the hyperbolic sine of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.sinh()))
    }

    fn sqrt(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is one of NaN, +0𝔽, -0𝔽, or +∞𝔽, return n.
        if n.is_nan() || n == 0.0 || n == f64::INFINITY {
            return Ok(n_number.into_value());
        }
        // 3. If n < -0𝔽, return NaN.
        if n < -0.0 {
            return Ok(Value::nan());
        }
        // 4. Return an implementation-approximated Number value representing the square root of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.sqrt()))
    }

    fn tan(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n.
        if n.is_nan() || n == 0.0 {
            return Ok(n_number.into_value());
        }
        // 3. If n is either +∞𝔽 or -∞𝔽, return NaN.
        if n.is_infinite() {
            return Ok(Value::nan());
        }
        // 4. Return an implementation-approximated Number value representing the tangent of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.tan()))
    }

    fn tanh(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);
        // 2. If n is one of NaN, +0𝔽, or -0𝔽, return n.
        if n.is_nan() || n == 0.0 {
            return Ok(n_number.into_value());
        }
        // 3. If n is +∞𝔽, return 1𝔽.
        if n == f64::INFINITY {
            return Ok(Value::from(1));
        }
        // 4. If n is -∞𝔽, return -1𝔽.
        if n == f64::NEG_INFINITY {
            return Ok(Value::from(-1));
        }
        // 5. Return an implementation-approximated Number value representing the hyperbolic tangent of ℝ(n).
        Ok(Value::from_f64(agent, gc.into_nogc(), n.tanh()))
    }

    fn trunc(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. Let n be ? ToNumber(x).
        let n_number = to_number(agent, gc.reborrow(), arguments.get(0))?;
        let n = n_number.into_f64(agent);

        // 2. If n is not finite or n is either +0𝔽 or -0𝔽, return n.
        if !n.is_finite() || n == 0.0 {
            return Ok(n_number.into_value());
        }

        // 3. If n < 1𝔽 and n > +0𝔽, return +0𝔽.
        if n < 1.0 && n > 0.0 {
            return Ok(Value::pos_zero());
        }

        // 4. If n < -0𝔽 and n > -1𝔽, return -0𝔽.
        if n < -0.0 && n > -1.0 {
            return Ok(Value::neg_zero());
        }

        // 5. Return the integral Number nearest n in the direction of +0𝔽.
        Ok(Value::from_f64(agent, gc.into_nogc(), n.trunc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, gc: NoGcScope, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.math();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(44)
            .with_prototype(object_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.E.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::E).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LN10.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::LN_10).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LN2.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::LN_2).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LOG10E.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::LOG10_E).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LOG2E.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::LOG2_E).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.PI.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::PI).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.SQRT1_2.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::FRAC_1_SQRT_2).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.SQRT2.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, gc, consts::SQRT_2).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_builtin_function_property::<MathObjectAbs>()
            .with_builtin_function_property::<MathObjectAcos>()
            .with_builtin_function_property::<MathObjectAcosh>()
            .with_builtin_function_property::<MathObjectAsin>()
            .with_builtin_function_property::<MathObjectAsinh>()
            .with_builtin_function_property::<MathObjectAtan>()
            .with_builtin_function_property::<MathObjectAtanh>()
            .with_builtin_function_property::<MathObjectAtan2>()
            .with_builtin_function_property::<MathObjectCbrt>()
            .with_builtin_function_property::<MathObjectCeil>()
            .with_builtin_function_property::<MathObjectClz32>()
            .with_builtin_function_property::<MathObjectCos>()
            .with_builtin_function_property::<MathObjectCosh>()
            .with_builtin_function_property::<MathObjectExp>()
            .with_builtin_function_property::<MathObjectExpm1>()
            .with_builtin_function_property::<MathObjectFloor>()
            .with_builtin_function_property::<MathObjectFround>()
            .with_builtin_function_property::<MathObjectHypot>()
            .with_builtin_function_property::<MathObjectImul>()
            .with_builtin_function_property::<MathObjectLog>()
            .with_builtin_function_property::<MathObjectLog1p>()
            .with_builtin_function_property::<MathObjectLog10>()
            .with_builtin_function_property::<MathObjectLog2>()
            .with_builtin_function_property::<MathObjectMax>()
            .with_builtin_function_property::<MathObjectMin>()
            .with_builtin_function_property::<MathObjectPow>()
            .with_builtin_function_property::<MathObjectRandom>()
            .with_builtin_function_property::<MathObjectRound>()
            .with_builtin_function_property::<MathObjectSign>()
            .with_builtin_function_property::<MathObjectSin>()
            .with_builtin_function_property::<MathObjectSinh>()
            .with_builtin_function_property::<MathObjectSqrt>()
            .with_builtin_function_property::<MathObjectTan>()
            .with_builtin_function_property::<MathObjectTanh>()
            .with_builtin_function_property::<MathObjectTrunc>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Math.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

/// Separate slow path for Number.prototype.max to take the heap allocation out
/// of the main body.
#[inline(never)]
fn max_slow_path(
    agent: &mut Agent,
    mut gc: GcScope,
    only_ints: &mut bool,
    highest_i64: &mut i64,
    highest_f64: &mut f64,
    arguments: &[Value],
) -> JsResult<bool> {
    // First gather remaining arguments into Vec and scope each one to
    // make them safe from GC.
    let remaining_arguments = arguments
        .iter()
        .map(|arg| arg.scope(agent, gc.nogc()))
        .collect::<Vec<_>>();
    let mut contained_nan = false;
    // Then pull the values down from the heap one by one and convert
    // them to Numbers, reducing their value into the current limit.
    for arg in remaining_arguments.into_iter() {
        // a. Let n be ? ToNumber(arg).
        let n = to_number(agent, gc.reborrow(), arg.get(agent))?;
        // b. Append n to coerced.

        if let Number::Integer(int) = n {
            let int = int.into_i64();
            *highest_i64 = (*highest_i64).max(int);
        } else {
            *only_ints = false;

            // 4. For each element number of coerced, do
            // a. If n is NaN, return NaN.
            if n.is_nan(agent) {
                contained_nan = true;
            } else {
                // b. If n is +0𝔽 and highest is -0𝔽, set highest to +0𝔽.
                // Note: This is handled automatically as +0 is integer.
                // c. If n > highest, set highest to n.
                *highest_f64 = (*highest_f64).max(n.into_f64(agent));
            }
        }
    }
    Ok(contained_nan)
}

#[inline(never)]
fn min_slow_path(
    agent: &mut Agent,
    mut gc: GcScope,
    only_ints: &mut bool,
    lowest_i64: &mut i64,
    lowest_f64: &mut f64,
    arguments: &[Value],
) -> JsResult<bool> {
    // First gather remaining arguments into Vec and scope each one to
    // make them safe from GC.
    let remaining_arguments = arguments
        .iter()
        .map(|arg| arg.scope(agent, gc.nogc()))
        .collect::<Vec<_>>();
    let mut contained_nan = false;
    // Then pull the values down from the heap one by one and convert
    // them to Numbers, reducing their value into the current limit.
    for arg in remaining_arguments.into_iter() {
        // a. Let n be ? ToNumber(arg).
        let number = to_number(agent, gc.reborrow(), arg.get(agent))?;
        // b. Append n to coerced.

        if let Number::Integer(int) = number {
            // c. If number < lowest, set lowest to number.
            *lowest_i64 = (*lowest_i64).min(int.into_i64());
        } else {
            *only_ints = false;

            let number = number.into_f64(agent);
            // a. If number is NaN, return NaN.
            if number.is_nan() {
                contained_nan = true;
            } else {
                // b. If number is -0𝔽 and lowest is +0𝔽, set lowest to -0𝔽.
                // Note: We'll handle this later. +0 is always an integer.
                // c. If number < lowest, set lowest to number.
                *lowest_f64 = (*lowest_f64).min(number);
            }
        }
    }
    Ok(contained_nan)
}
