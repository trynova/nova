use super::{
    builtin_function::{define_builtin_function, define_builtin_property},
    create_builtin_function, todo_builtin, ArgumentsList, Behaviour, Builtin, BuiltinFunctionArgs,
};
use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{Number, Object, PropertyDescriptor, Value},
    },
    heap::CreateHeapData,
    SmallInteger,
};

pub struct NumberConstructor;

impl Builtin for NumberConstructor {
    fn create<'a>(agent: &'a mut Agent<'a, 'a>) -> JsResult<Object> {
        let realm_id = agent.current_realm_id();

        let function_prototype = agent.current_realm().intrinsics().function_prototype();
        let number_prototype = agent.current_realm().intrinsics().number_prototype();
        let object: Object = create_builtin_function(
            agent,
            Behaviour::Constructor(Self::behaviour),
            BuiltinFunctionArgs {
                length: 1,
                name: "Number",
                realm: Some(realm_id),
                prototype: Some(function_prototype),
                ..Default::default()
            },
        )
        .into_object();

        // 21.1.2.1 Number.EPSILON
        // https://tc39.es/ecma262/#sec-number.epsilon
        define_builtin_property(
            object,
            "EPSILON",
            PropertyDescriptor {
                value: Some(agent.heap.create(f64::EPSILON).into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.2.6 Number.MAX_SAFE_INTEGER
        // https://tc39.es/ecma262/#sec-number.max_safe_integer
        define_builtin_property(
            object,
            "MAX_SAFE_INTEGER",
            PropertyDescriptor {
                value: Some(Number::from(SmallInteger::MAX_NUMBER).into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.2.7 Number.MAX_VALUE
        // https://tc39.es/ecma262/#sec-number.max_value
        define_builtin_property(
            object,
            "MAX_VALUE",
            PropertyDescriptor {
                value: Some(agent.heap.create(f64::MAX).into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.2.8 Number.MIN_SAFE_INTEGER
        // https://tc39.es/ecma262/#sec-number.min_safe_integer
        define_builtin_property(
            object,
            "MIN_SAFE_INTEGER",
            PropertyDescriptor {
                value: Some(Number::from(SmallInteger::MIN_NUMBER).into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.2.8 Number.MIN_VALUE
        // https://tc39.es/ecma262/#sec-number.min_value
        define_builtin_property(
            object,
            "MIN_VALUE",
            PropertyDescriptor {
                value: Some(agent.heap.create(f64::MIN).into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.2.10 Number.NaN
        // https://tc39.es/ecma262/#sec-number.nan
        define_builtin_property(
            object,
            "NaN",
            PropertyDescriptor {
                value: Some(Number::nan().into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.2.11 Number.NEGATIVE_INFINITY
        // https://tc39.es/ecma262/#sec-number.negative_infinity
        define_builtin_property(
            object,
            "NEGATIVE_INFINITY",
            PropertyDescriptor {
                value: Some(Number::neg_inf().into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.2.14 Number.POSITIVE_INFINITY
        // https://tc39.es/ecma262/#sec-number.positive_infinity
        define_builtin_property(
            object,
            "POSITIVE_INFINITY",
            PropertyDescriptor {
                value: Some(Number::pos_inf().into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        define_builtin_function(agent, object, "isFinite", todo_builtin, 1, realm_id)?;
        define_builtin_function(agent, object, "isNaN", todo_builtin, 1, realm_id)?;
        define_builtin_function(agent, object, "isSafeInteger", todo_builtin, 1, realm_id)?;
        define_builtin_function(agent, object, "parseFloat", todo_builtin, 1, realm_id)?;
        define_builtin_function(agent, object, "parseInt", todo_builtin, 2, realm_id)?;

        // 21.1.2.15 Number.prototype
        // https://tc39.es/ecma262/#sec-number.prototype
        define_builtin_property(
            object,
            "prototype",
            PropertyDescriptor {
                value: Some(number_prototype.into_value()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(false),
                ..Default::default()
            },
        )?;

        // 21.1.3.1 Number.prototype.constructor
        // https://tc39.es/ecma262/#sec-number.prototype.constructor
        define_builtin_property(
            number_prototype,
            "constructor",
            PropertyDescriptor {
                value: Some(object.into_value()),
                writable: Some(true),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            },
        )?;

        Ok(object)
    }
}

impl NumberConstructor {
    /// 21.1.1.1 Number ( value )
    /// https://tc39.es/ecma262/#sec-number-constructor-number-value
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
}
