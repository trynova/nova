use std::f64::consts;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_number,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{GetHeapData, WellKnownSymbolIndexes},
    SmallInteger,
};

pub(crate) struct MathObject;

struct MathObjectAbs;
impl Builtin for MathObjectAbs {
    const NAME: String = BUILTIN_STRING_MEMORY.abs;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::abs);
}

struct MathObjectAcos;
impl Builtin for MathObjectAcos {
    const NAME: String = BUILTIN_STRING_MEMORY.acos;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::acos);
}
struct MathObjectAcosh;
impl Builtin for MathObjectAcosh {
    const NAME: String = BUILTIN_STRING_MEMORY.acosh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::acosh);
}
struct MathObjectAsin;
impl Builtin for MathObjectAsin {
    const NAME: String = BUILTIN_STRING_MEMORY.asin;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::asin);
}
struct MathObjectAsinh;
impl Builtin for MathObjectAsinh {
    const NAME: String = BUILTIN_STRING_MEMORY.asinh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::asinh);
}
struct MathObjectAtan;
impl Builtin for MathObjectAtan {
    const NAME: String = BUILTIN_STRING_MEMORY.atan;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::atan);
}
struct MathObjectAtanh;
impl Builtin for MathObjectAtanh {
    const NAME: String = BUILTIN_STRING_MEMORY.atanh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::atanh);
}
struct MathObjectAtan2;
impl Builtin for MathObjectAtan2 {
    const NAME: String = BUILTIN_STRING_MEMORY.atan2;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::atan2);
}
struct MathObjectCbrt;
impl Builtin for MathObjectCbrt {
    const NAME: String = BUILTIN_STRING_MEMORY.cbrt;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::cbrt);
}
struct MathObjectCeil;
impl Builtin for MathObjectCeil {
    const NAME: String = BUILTIN_STRING_MEMORY.ceil;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::ceil);
}
struct MathObjectClz32;
impl Builtin for MathObjectClz32 {
    const NAME: String = BUILTIN_STRING_MEMORY.clz32;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::clz32);
}
struct MathObjectCos;
impl Builtin for MathObjectCos {
    const NAME: String = BUILTIN_STRING_MEMORY.cos;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::cos);
}
struct MathObjectCosh;
impl Builtin for MathObjectCosh {
    const NAME: String = BUILTIN_STRING_MEMORY.cosh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::cosh);
}
struct MathObjectExp;
impl Builtin for MathObjectExp {
    const NAME: String = BUILTIN_STRING_MEMORY.exp;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::exp);
}
struct MathObjectExpm1;
impl Builtin for MathObjectExpm1 {
    const NAME: String = BUILTIN_STRING_MEMORY.expm1;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::expm1);
}
struct MathObjectFloor;
impl Builtin for MathObjectFloor {
    const NAME: String = BUILTIN_STRING_MEMORY.floor;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::floor);
}
struct MathObjectFround;
impl Builtin for MathObjectFround {
    const NAME: String = BUILTIN_STRING_MEMORY.fround;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::fround);
}
struct MathObjectHypot;
impl Builtin for MathObjectHypot {
    const NAME: String = BUILTIN_STRING_MEMORY.hypot;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::hypot);
}
struct MathObjectImul;
impl Builtin for MathObjectImul {
    const NAME: String = BUILTIN_STRING_MEMORY.imul;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::imul);
}
struct MathObjectLog;
impl Builtin for MathObjectLog {
    const NAME: String = BUILTIN_STRING_MEMORY.log;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log);
}
struct MathObjectLog1p;
impl Builtin for MathObjectLog1p {
    const NAME: String = BUILTIN_STRING_MEMORY.log1p;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log1p);
}
struct MathObjectLog10;
impl Builtin for MathObjectLog10 {
    const NAME: String = BUILTIN_STRING_MEMORY.log10;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log10);
}
struct MathObjectLog2;
impl Builtin for MathObjectLog2 {
    const NAME: String = BUILTIN_STRING_MEMORY.log2;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::log2);
}
struct MathObjectMax;
impl Builtin for MathObjectMax {
    const NAME: String = BUILTIN_STRING_MEMORY.max;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::max);
}
struct MathObjectMin;
impl Builtin for MathObjectMin {
    const NAME: String = BUILTIN_STRING_MEMORY.min;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::min);
}
struct MathObjectPow;
impl Builtin for MathObjectPow {
    const NAME: String = BUILTIN_STRING_MEMORY.pow;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::pow);
}
struct MathObjectRandom;
impl Builtin for MathObjectRandom {
    const NAME: String = BUILTIN_STRING_MEMORY.random;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::random);
}
struct MathObjectRound;
impl Builtin for MathObjectRound {
    const NAME: String = BUILTIN_STRING_MEMORY.round;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::round);
}
struct MathObjectSign;
impl Builtin for MathObjectSign {
    const NAME: String = BUILTIN_STRING_MEMORY.sign;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sign);
}
struct MathObjectSin;
impl Builtin for MathObjectSin {
    const NAME: String = BUILTIN_STRING_MEMORY.sin;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sin);
}
struct MathObjectSinh;
impl Builtin for MathObjectSinh {
    const NAME: String = BUILTIN_STRING_MEMORY.sinh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sinh);
}
struct MathObjectSqrt;
impl Builtin for MathObjectSqrt {
    const NAME: String = BUILTIN_STRING_MEMORY.sqrt;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::sqrt);
}
struct MathObjectTan;
impl Builtin for MathObjectTan {
    const NAME: String = BUILTIN_STRING_MEMORY.tan;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::tan);
}
struct MathObjectTanh;
impl Builtin for MathObjectTanh {
    const NAME: String = BUILTIN_STRING_MEMORY.tanh;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::tanh);
}
struct MathObjectTrunc;
impl Builtin for MathObjectTrunc {
    const NAME: String = BUILTIN_STRING_MEMORY.trunc;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(MathObject::trunc);
}

impl MathObject {
    fn abs(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let n = to_number(agent, arguments.get(0))?;
        match n {
            Number::Number(idx) => {
                let data = *agent.heap.get(idx);
                // NaN, -0, and infinites should be handled in Float
                debug_assert!(!data.is_nan() && data != -0.0 && !data.is_infinite());
                if data.is_sign_negative() {
                    Ok(Number::from_f64(agent, data.abs()).into_value())
                } else {
                    Ok(n.into_value())
                }
            }
            Number::Integer(int) => {
                let int = int.into_i64();
                if int.is_negative() {
                    Ok(Number::from(SmallInteger::try_from(int.abs()).unwrap()).into_value())
                } else {
                    Ok(n.into_value())
                }
            }
            Number::Float(_) => todo!(),
        }
    }

    fn acos(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn acosh(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn asin(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn asinh(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn atan(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn atanh(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn atan2(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn cbrt(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn ceil(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn clz32(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn cos(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn cosh(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn exp(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn expm1(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn floor(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn fround(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn hypot(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn imul(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn log(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn log1p(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn log10(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn log2(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn max(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn min(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn pow(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _base = arguments.get(0);
        let _exponent = arguments.get(0);
        todo!();
    }

    fn random(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn round(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn sign(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn sin(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn sinh(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn sqrt(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn tan(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn tanh(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    fn trunc(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let _x = to_number(agent, arguments.get(0))?;
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.math();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(46)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.E.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::E).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LN10.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::LN_10).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LN2.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::LN_2).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LOG10E.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::LOG10_E).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.LOG2E.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::LOG2_E).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.PI.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::PI).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.SQRT1_2.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::FRAC_1_SQRT_2).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.SQRT2.into())
                    .with_value_creator_readonly(|agent| {
                        Number::from_f64(agent, consts::SQRT_2).into_value()
                    })
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.Math.into())
                    .with_value_readonly(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_enumerable(false)
                    .with_configurable(true)
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
            .build();
    }
}
