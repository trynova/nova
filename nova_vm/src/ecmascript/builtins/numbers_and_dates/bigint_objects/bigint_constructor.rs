use num_bigint::ToBigInt;

use crate::ecmascript::abstract_operations::testing_and_comparison::is_integral_number;
use crate::ecmascript::abstract_operations::type_conversion::to_big_int;
use crate::ecmascript::abstract_operations::type_conversion::to_index;
use crate::ecmascript::abstract_operations::type_conversion::to_primitive;
use crate::ecmascript::abstract_operations::type_conversion::PreferredType;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::BigInt;
use crate::ecmascript::types::BigIntHeapData;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Number;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::{String, Value};
use crate::heap::indexes::BigIntIndex;

use crate::heap::GetHeapData;
use crate::SmallInteger;

/// ### [21.1.2.1 BigInt ( value )](https://tc39.es/ecma262/#sec-bigint-constructor)
pub struct BigIntConstructor;

impl Builtin for BigIntConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.BigInt;
}

struct BigIntAsIntN;
impl Builtin for BigIntAsIntN {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(BigIntConstructor::as_int_n);
    const LENGTH: u8 = 2;
    const NAME: String = BUILTIN_STRING_MEMORY.asIntN;
}
struct BigIntAsUintN;
impl Builtin for BigIntAsUintN {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(BigIntConstructor::as_uint_n);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.asUintN;
}

impl BigIntConstructor {
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        if new_target.is_some() {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "BigInt is not a constructor")
            );
        }
        let value = arguments.get(0);
        let prim = to_primitive(agent, value, Some(PreferredType::Number))?;
        if let Ok(prim) = Number::try_from(prim) {
            Ok(prim.into_value())
        } else {
            to_big_int(agent, value).map(|result| result.into_value())
        }
    }

    fn as_int_n(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let bits = to_index(agent, arguments.get(0))?;
        if bits as u32 as i64 != bits {
            return Err(agent.throw_exception(
                ExceptionType::RangeError,
                "Ridiculous bits value for BigInt.asIntN",
            ));
        }
        let bits = bits as u32;
        let bigint = to_big_int(agent, arguments.get(1))?;
        match bigint {
            BigInt::BigInt(_) => todo!(),
            BigInt::SmallBigInt(int) => {
                let int = int.into_i64();
                let modulo = int % 2i64.pow(bits);
                if modulo >= 2i64.pow(bits - 1) {
                    Ok(
                        BigInt::from(SmallInteger::try_from(modulo - 2i64.pow(bits)).unwrap())
                            .into_value(),
                    )
                } else {
                    Ok(BigInt::from(SmallInteger::try_from(modulo).unwrap()).into_value())
                }
            }
        }
    }

    fn as_uint_n(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let bits = to_index(agent, arguments.get(0))?;
        if bits as u32 as i64 != bits {
            return Err(agent.throw_exception(
                ExceptionType::RangeError,
                "Ridiculous bits value for BigInt.asUintN",
            ));
        }
        let bits = bits as u32;
        let bigint = to_big_int(agent, arguments.get(1))?;
        match bigint {
            BigInt::BigInt(_) => todo!(),
            BigInt::SmallBigInt(int) => {
                let int = int.into_i64();
                let modulo = int % 2i64.pow(bits);
                Ok(BigInt::from(SmallInteger::try_from(modulo).unwrap()).into_value())
            }
        }
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let big_int_prototype = intrinsics.big_int_prototype();
        let this = intrinsics.big_int();
        let this_object_index = intrinsics.big_int_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<BigIntConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(3)
        .with_builtin_function_property::<BigIntAsIntN>()
        .with_builtin_function_property::<BigIntAsUintN>()
        .with_prototype_property(big_int_prototype.into_object())
        .build();
    }
}

fn number_to_big_int(agent: &mut Agent, value: Number) -> JsResult<BigInt> {
    if !is_integral_number(agent, value) {
        Err(agent.throw_exception(ExceptionType::RangeError, "Not an integer"))
    } else {
        match value {
            Number::Number(idx) => {
                let value = *agent.heap.get(idx);
                if let Ok(data) = SmallInteger::try_from(value) {
                    Ok(data.into())
                } else {
                    let number = value.to_bigint().unwrap();
                    agent
                        .heap
                        .bigints
                        .push(Some(BigIntHeapData { data: number }));
                    Ok(BigIntIndex::last(&agent.heap.bigints).into())
                }
            }
            Number::Integer(int) => Ok(int.into()),
            Number::Float(value) => {
                if let Ok(data) = SmallInteger::try_from(value) {
                    Ok(data.into())
                } else {
                    let number = value.to_bigint().unwrap();
                    agent
                        .heap
                        .bigints
                        .push(Some(BigIntHeapData { data: number }));
                    Ok(BigIntIndex::last(&agent.heap.bigints).into())
                }
            }
        }
    }
}
