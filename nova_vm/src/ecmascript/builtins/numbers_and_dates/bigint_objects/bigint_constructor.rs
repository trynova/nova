// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use num_traits::Pow;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            PreferredType, to_big_int, to_big_int_primitive, to_index, to_primitive,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, BigInt, Number, Object, String, Value},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
        small_bigint::SmallBigInt,
    },
    heap::{ArenaAccess, IntrinsicConstructorIndexes},
};

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
    /// ### [21.2.1.1 BigInt ( value )](https://tc39.es/ecma262/#sec-bigint-constructor-number-value)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let value = arguments.get(0).bind(gc.nogc());

        // 1. If NewTarget is not undefined,
        if new_target.is_some() {
            // throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "BigInt is not a constructor",
                gc.into_nogc(),
            ));
        }
        // 2. Let prim be ? ToPrimitive(value, number).
        let prim = to_primitive(
            agent,
            value.unbind(),
            Some(PreferredType::Number),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 3. If prim is a Number,
        if let Ok(prim) = Number::try_from(prim) {
            // return ? NumberToBigInt(prim).
            if !prim.is_integer_(agent) {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "Can't convert number to BigInt because it isn't an integer",
                    gc.into_nogc(),
                ));
            }

            Ok(BigInt::from_i64(agent, prim.into_i64_(agent)).into())
        } else {
            // 4. Otherwise, return ? ToBigInt(prim).
            to_big_int_primitive(agent, prim.unbind(), gc.into_nogc()).map(|result| result.into())
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
            return Ok(BigInt::zero().into());
        }

        match 2i64.checked_pow(bits) {
            Some(divisor) => {
                match bigint {
                    BigInt::BigInt(bigint) => {
                        let modulo = &bigint.get(agent).data % divisor;
                        // SAFETY: This cannot overflow since 2^bits didn't.
                        let divisor_half = divisor >> 1;
                        if let Ok(modulo) = i64::try_from(&modulo) {
                            let modulo = if modulo >= divisor_half {
                                modulo - divisor
                            } else {
                                modulo
                            };
                            Ok(BigInt::from_i64(agent, modulo).into())
                        } else {
                            Ok(BigInt::from_num_bigint(agent, modulo - divisor).into())
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
                        Ok(BigInt::from(SmallBigInt::try_from(modulo).unwrap()).into())
                    }
                }
            }
            _ => {
                let divisor =
                    num_bigint::BigInt::from_bytes_le(num_bigint::Sign::Plus, &[2]).pow(bits);
                match bigint {
                    BigInt::BigInt(bigint) => {
                        let modulo = &bigint.get(agent).data % &divisor;
                        let divisor_half = &divisor >> 1;
                        if let Ok(modulo) = i64::try_from(&modulo) {
                            // Maybe safe? Maybe not.
                            Ok(BigInt::from_i64(agent, modulo).into())
                        } else {
                            let modulo = if modulo >= divisor_half {
                                modulo - divisor
                            } else {
                                modulo
                            };
                            Ok(BigInt::from_num_bigint(agent, modulo).into())
                        }
                    }
                    BigInt::SmallBigInt(_) => {
                        // Probably safe: The divisor is bigger than i64 but
                        // value is i54.
                        Ok(bigint.unbind().into())
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

        match 2i64.checked_pow(bits) {
            Some(modulus) => match bigint {
                BigInt::BigInt(int) => {
                    let int = &int.get(agent).data;
                    Ok(
                        BigInt::from_num_bigint(agent, ((int % modulus) + modulus) % modulus)
                            .into(),
                    )
                }
                BigInt::SmallBigInt(int) => {
                    let int = int.into_i64();
                    let modulo = int.rem_euclid(modulus);
                    Ok(BigInt::from(SmallBigInt::try_from(modulo).unwrap()).into())
                }
            },
            None => {
                let modulus =
                    num_bigint::BigInt::from_bytes_le(num_bigint::Sign::Plus, &[2]).pow(bits);
                match bigint {
                    BigInt::BigInt(int) => {
                        let int = &int.get(agent).data;
                        let result = ((int % &modulus) + &modulus) % &modulus;
                        Ok(BigInt::from_num_bigint(agent, result).into())
                    }
                    BigInt::SmallBigInt(int) => {
                        let int = int.into_i64();
                        let result = ((int % &modulus) + &modulus) % &modulus;
                        Ok(BigInt::from_num_bigint(agent, result).into())
                    }
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
            .with_prototype_property(big_int_prototype.into())
            .build();
    }
}
