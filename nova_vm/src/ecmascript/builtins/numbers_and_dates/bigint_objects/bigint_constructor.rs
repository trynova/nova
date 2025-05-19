// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use num_bigint::ToBigInt;
use num_traits::Pow;

use crate::ecmascript::abstract_operations::testing_and_comparison::is_integral_number;
use crate::ecmascript::abstract_operations::type_conversion::PreferredType;
use crate::ecmascript::abstract_operations::type_conversion::to_big_int;
use crate::ecmascript::abstract_operations::type_conversion::to_big_int_primitive;
use crate::ecmascript::abstract_operations::type_conversion::to_index;
use crate::ecmascript::abstract_operations::type_conversion::to_primitive;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::Realm;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::BigInt;
use crate::ecmascript::types::BigIntHeapData;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Number;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::{String, Value};

use crate::SmallInteger;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::rootable::Scopable;
use crate::engine::small_bigint::SmallBigInt;
use crate::heap::CreateHeapData;
use crate::heap::IntrinsicConstructorIndexes;

/// ### [21.1.2.1 BigInt ( value )](https://tc39.es/ecma262/#sec-bigint-constructor)
pub struct BigIntConstructor;

impl Builtin for BigIntConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.BigInt;
}
impl BuiltinIntrinsicConstructor for BigIntConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::BigInt;
}

struct BigIntAsIntN;
impl Builtin for BigIntAsIntN {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(BigIntConstructor::as_int_n);
    const LENGTH: u8 = 2;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.asIntN;
}
struct BigIntAsUintN;
impl Builtin for BigIntAsUintN {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(BigIntConstructor::as_uint_n);
    const LENGTH: u8 = 2;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.asUintN;
}

impl BigIntConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        if new_target.is_some() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "BigInt is not a constructor",
                gc.into_nogc(),
            ));
        }
        let value = arguments.get(0).bind(gc.nogc());
        let prim = to_primitive(
            agent,
            value.unbind(),
            Some(PreferredType::Number),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        if let Ok(prim) = Number::try_from(prim) {
            if !prim.is_integer(agent) {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "Can't convert number to BigInt because it isn't an integer",
                    gc.into_nogc(),
                ));
            }

            Ok(BigInt::from_i64(agent, prim.into_i64(agent)).into_value())
        } else {
            to_big_int_primitive(agent, prim.unbind(), gc.into_nogc())
                .map(|result| result.into_value())
        }
    }

    /// ### [21.2.2.1 BigInt.asIntN ( bits, bigint )](https://tc39.es/ecma262/#sec-bigint.asintn)
    fn as_int_n<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let bits = arguments.get(0).bind(gc.nogc());
        let bigint = arguments.get(1).scope(agent, gc.nogc());
        let bits = to_index(agent, bits.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let Ok(bits) = u32::try_from(bits) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Ridiculous bits value for BigInt.asIntN",
                gc.into_nogc(),
            ));
        };
        let bigint = to_big_int(agent, bigint.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        if bits == 0 {
            return Ok(BigInt::zero().into_value());
        }

        match 2i64.checked_pow(bits) {
            Some(divisor) => {
                match bigint {
                    BigInt::BigInt(bigint) => {
                        let modulo = &agent[bigint].data % divisor;
                        // SAFETY: This cannot overflow since 2^bits didn't.
                        let divisor_half = divisor >> 1;
                        if let Ok(modulo) = i64::try_from(&modulo) {
                            let modulo = if modulo >= divisor_half {
                                modulo - divisor
                            } else {
                                modulo
                            };
                            Ok(BigInt::from_i64(agent, modulo).into_value())
                        } else {
                            Ok(BigInt::from_num_bigint(agent, modulo - divisor).into_value())
                        }
                    }
                    BigInt::SmallBigInt(bigint) => {
                        let bigint = bigint.into_i64();
                        let modulo = bigint.rem_euclid(divisor);
                        let modulo = if modulo >= 2i64.pow(bits - 1) {
                            modulo - divisor
                        } else {
                            modulo
                        };
                        Ok(BigInt::from(SmallBigInt::try_from(modulo).unwrap()).into_value())
                    }
                }
            }
            _ => {
                let divisor =
                    num_bigint::BigInt::from_bytes_le(num_bigint::Sign::Plus, &[2]).pow(bits);
                match bigint {
                    BigInt::BigInt(bigint) => {
                        let modulo = &agent[bigint].data % &divisor;
                        let divisor_half = &divisor >> 1;
                        if let Ok(modulo) = i64::try_from(&modulo) {
                            // Maybe safe? Maybe not.
                            Ok(BigInt::from_i64(agent, modulo).into_value())
                        } else {
                            let modulo = if modulo >= divisor_half {
                                modulo - divisor
                            } else {
                                modulo
                            };
                            Ok(BigInt::from_num_bigint(agent, modulo).into_value())
                        }
                    }
                    BigInt::SmallBigInt(_) => {
                        // Probably safe: The divisor is bigger than i64 but
                        // value is i54.
                        Ok(bigint.into_value().unbind())
                    }
                }
            }
        }
    }

    /// ### [21.2.2.2 BigInt.asUintN ( bits, bigint )](https://tc39.es/ecma262/#sec-bigint.asuintn)
    fn as_uint_n<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let bits = arguments.get(0).bind(gc.nogc());
        let bigint = arguments.get(1).scope(agent, gc.nogc());
        let bits = to_index(agent, bits.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        let Ok(bits) = u32::try_from(bits) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Ridiculous bits value for BigInt.asUintN",
                gc.into_nogc(),
            ));
        };
        let bigint = to_big_int(agent, bigint.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        match bigint {
            BigInt::BigInt(int) => {
                let int = &agent[int].data;
                let modulus = 2i64.pow(bits);
                Ok(
                    BigInt::from_num_bigint(agent, ((int % modulus) + modulus) % modulus)
                        .into_value(),
                )
            }
            BigInt::SmallBigInt(int) => {
                let int = int.into_i64();
                if let Some(modulo) = 2i64
                    .checked_pow(bits)
                    .and_then(|base| int.checked_rem_euclid(base))
                {
                    Ok(BigInt::from(SmallBigInt::try_from(modulo).unwrap()).into_value())
                } else {
                    let modulus = num_bigint::BigInt::from(2).pow(bits);
                    let result = ((int % modulus.clone()) + modulus.clone()) % modulus;
                    Ok(BigInt::from_num_bigint(agent, result).into_value())
                }
            }
        }
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let big_int_prototype = intrinsics.big_int_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<BigIntConstructor>(agent, realm)
            .with_property_capacity(3)
            .with_builtin_function_property::<BigIntAsIntN>()
            .with_builtin_function_property::<BigIntAsUintN>()
            .with_prototype_property(big_int_prototype.into_object())
            .build();
    }
}

fn number_to_big_int<'a>(
    agent: &mut Agent,
    value: Number<'a>,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, BigInt<'a>> {
    let gc = gc.into_nogc();
    if !is_integral_number(agent, value) {
        Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Not an integer",
            gc,
        ))
    } else {
        match value {
            Number::Number(idx) => {
                let value = agent[idx];
                if let Ok(data) = SmallInteger::try_from(value) {
                    Ok(BigInt::SmallBigInt(data.into()))
                } else {
                    let number = value.to_bigint().unwrap();
                    Ok(agent.heap.create(BigIntHeapData { data: number }))
                }
            }
            Number::Integer(int) => Ok(BigInt::SmallBigInt(int.into())),
            Number::SmallF64(value) => {
                let value = value.into_f64();
                if let Ok(data) = SmallInteger::try_from(value) {
                    Ok(BigInt::SmallBigInt(data.into()))
                } else {
                    let number = value.to_bigint().unwrap();
                    Ok(agent.heap.create(BigIntHeapData { data: number }))
                }
            }
        }
    }
}
