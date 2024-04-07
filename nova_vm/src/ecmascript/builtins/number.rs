use super::{ArgumentsList, Behaviour, Builtin};
use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::is_integral_number,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, IntoValue, Number, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::CreateHeapData,
    SmallInteger,
};

/// ### [21.1.1.1 Number ( value )](https://tc39.es/ecma262/#sec-number-constructor-number-value)
pub struct NumberConstructor;

impl Builtin for NumberConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.Number;
}

struct NumberIsFinite;
impl Builtin for NumberIsFinite {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberConstructor::is_finite);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.isFinite;
}
struct NumberIsInteger;
impl Builtin for NumberIsInteger {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberConstructor::is_integer);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.isInteger;
}
struct NumberIsNaN;
impl Builtin for NumberIsNaN {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberConstructor::is_nan);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.isNaN;
}
struct NumberIsSafeInteger;
impl Builtin for NumberIsSafeInteger {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(NumberConstructor::is_safe_integer);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.isSafeInteger;
}

impl NumberConstructor {
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let value = arguments.get(0);

        // 1. If value is present, then
        let n = if !value.is_undefined() {
            // a. Let prim be ? ToNumeric(value).
            let prim = value.to_numeric(agent)?;

            // b. If prim is a BigInt, let n be 𝔽(ℝ(prim)).
            if prim.is_bigint() {
                todo!()
            }
            // c. Otherwise, let n be prim.
            else {
                prim
            }
        }
        // 2. Else,
        else {
            // a. Let n be +0𝔽.
            Value::from(0)
        };

        // 3. If NewTarget is undefined, return n.
        let Some(_new_target) = new_target else {
            return Ok(n);
        };

        todo!();

        // 4. Let O be ? OrdinaryCreateFromConstructor(NewTarget, "%Number.prototype%", « [[NumberData]] »).
        // 5. Set O.[[NumberData]] to n.
        // 6. Return O.
    }

    /// ### [21.1.2.2 Number.isFinite ( number )](https://tc39.es/ecma262/#sec-number.isfinite)
    fn is_finite(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let maybe_number = arguments.get(0);

        // 1. If number is not a Number, return false.
        let Ok(number) = Number::try_from(maybe_number) else {
            return Ok(false.into());
        };

        // 2. If number is not finite, return false.
        // 3. Otherwise, return true.
        Ok(number.is_finite(agent).into())
    }

    /// ### [21.1.2.3 Number.isInteger ( number )](https://tc39.es/ecma262/#sec-number.isinteger)
    fn is_integer(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let maybe_number = arguments.get(0);

        // 1. Return IsIntegralNumber(number).
        Ok(is_integral_number(agent, maybe_number).into())
    }

    /// ### [21.1.2.4 Number.isNaN ( number )](https://tc39.es/ecma262/#sec-number.isnan)
    fn is_nan(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let maybe_number = arguments.get(0);

        // 1. If number is not a Number, return false.
        let Ok(number) = Number::try_from(maybe_number) else {
            return Ok(false.into());
        };

        // 2. If number is NaN, return true.
        // 3. Otherwise, return false.
        Ok(number.is_nan(agent).into())
    }

    /// ### [21.1.2.5 Number.isSafeInteger ( number )](https://tc39.es/ecma262/#sec-number.issafeinteger)
    fn is_safe_integer(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let maybe_number = arguments.get(0);

        // 1. If IsIntegralNumber(number) is true, then
        //    a. If abs(ℝ(number)) ≤ 2**53 - 1, return true.
        // 2. Return false.
        // NOTE: Integers must be stored in `Value::Integer`.
        Ok(matches!(maybe_number, Value::Integer(_)).into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let number_prototype = intrinsics.number_prototype();
        let parse_float = intrinsics.parse_float().into_value();
        let parse_int = intrinsics.parse_int().into_value();
        let this = intrinsics.number();
        let this_object_index = intrinsics.number_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<NumberConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(16)
        .with_property(|builder| {
            // 21.1.2.1 Number.EPSILON
            // https://tc39.es/ecma262/#sec-number.epsilon
            let value = Value::from_f64(builder.agent, f64::EPSILON);
            builder
                .with_key(BUILTIN_STRING_MEMORY.EPSILON.into())
                .with_value_readonly(value)
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        // 21.1.2.2 Number.isFinite ( number )
        .with_builtin_function_property::<NumberIsFinite>()
        // 21.1.2.3 Number.isInteger ( number )
        .with_builtin_function_property::<NumberIsInteger>()
        // 21.1.2.4 Number.isNaN ( number )
        .with_builtin_function_property::<NumberIsNaN>()
        // 21.1.2.5 Number.isSafeInteger ( number )
        .with_builtin_function_property::<NumberIsSafeInteger>()
        .with_property(|builder| {
            // 21.1.2.6 Number.MAX_SAFE_INTEGER
            // https://tc39.es/ecma262/#sec-number.max_safe_integer
            builder
                .with_key(BUILTIN_STRING_MEMORY.MAX_SAFE_INTEGER.into())
                .with_value_readonly(Number::from(SmallInteger::MAX_NUMBER).into())
                .with_configurable(false)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.7 Number.MAX_VALUE
            // https://tc39.es/ecma262/#sec-number.max_value
            builder
                .with_key(BUILTIN_STRING_MEMORY.MAX_VALUE.into())
                .with_value_creator_readonly(|agent| agent.heap.create(f64::MAX).into())
                .with_configurable(false)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.8 Number.MIN_SAFE_INTEGER
            // https://tc39.es/ecma262/#sec-number.min_safe_integer
            builder
                .with_key(BUILTIN_STRING_MEMORY.MIN_SAFE_INTEGER.into())
                .with_value_readonly(Number::from(SmallInteger::MIN_NUMBER).into())
                .with_configurable(false)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.8 Number.MIN_VALUE
            // https://tc39.es/ecma262/#sec-number.min_value
            builder
                .with_key(BUILTIN_STRING_MEMORY.MIN_VALUE.into())
                .with_value_creator_readonly(|agent| agent.heap.create(f64::MIN).into())
                .with_configurable(false)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.10 Number.NaN
            // https://tc39.es/ecma262/#sec-number.nan
            builder
                .with_key(BUILTIN_STRING_MEMORY.NaN.into())
                .with_value_readonly(Number::nan().into())
                .with_configurable(false)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.11 Number.NEGATIVE_INFINITY
            // https://tc39.es/ecma262/#sec-number.negative_infinity
            builder
                .with_key(BUILTIN_STRING_MEMORY.NEGATIVE_INFINITY.into())
                .with_value_readonly(Number::neg_inf().into())
                .with_configurable(false)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.12 Number.parseFloat ( string )
            builder
                .with_key(BUILTIN_STRING_MEMORY.parseFloat.into())
                .with_value(parse_float)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.13 Number.parseInt ( string, radix )
            builder
                .with_key(BUILTIN_STRING_MEMORY.parseInt.into())
                .with_value(parse_int)
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            // 21.1.2.14 Number.POSITIVE_INFINITY
            // https://tc39.es/ecma262/#sec-number.positive_infinity
            builder
                .with_key(BUILTIN_STRING_MEMORY.POSITIVE_INFINITY.into())
                .with_value_readonly(Number::pos_inf().into())
                .with_configurable(false)
                .with_enumerable(false)
                .build()
        })
        // 21.1.2.15 Number.prototype
        // https://tc39.es/ecma262/#sec-number.prototype
        .with_prototype_property(number_prototype.into_object())
        .build();
    }
}
