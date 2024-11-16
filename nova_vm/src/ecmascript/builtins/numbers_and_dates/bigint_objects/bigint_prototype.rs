// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{primitive_objects::PrimitiveObjectData, ArgumentsList, Builtin},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{BigInt, IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct BigIntPrototype;

struct BigIntPrototypeToLocaleString;
impl Builtin for BigIntPrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BigIntPrototype::to_locale_string);
}

struct BigIntPrototypeToString;
impl Builtin for BigIntPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BigIntPrototype::to_string);
}

struct BigIntPrototypeValueOf;
impl Builtin for BigIntPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BigIntPrototype::value_of);
}

impl BigIntPrototype {
    fn to_locale_string(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Self::to_string(agent, gc, this_value, arguments)
    }

    fn to_string(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let _x = this_big_int_value(agent, *gc, this_value)?;
        let radix = arguments.get(0);
        if radix.is_undefined() || radix == Value::from(10u8) {
            // BigInt::to_string_radix_10(agent, x).map(|result| result.into_value())
            todo!();
        } else {
            todo!();
        }
    }

    fn value_of(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        this_big_int_value(agent, *gc, this_value).map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.big_int_prototype();
        let big_int_constructor = intrinsics.big_int();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(object_prototype)
            .with_constructor_property(big_int_constructor)
            .with_builtin_function_property::<BigIntPrototypeToLocaleString>()
            .with_builtin_function_property::<BigIntPrototypeToString>()
            .with_builtin_function_property::<BigIntPrototypeValueOf>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.BigInt.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

/// ### [21.2.3.4.1 ThisBigIntValue ( value )](https://tc39.es/ecma262/#sec-thisbigintvalue)
///
/// The abstract operation ThisBigIntValue takes argument value (an ECMAScript
/// language value) and returns either a normal completion containing a BigInt
/// or a throw completion.
fn this_big_int_value(agent: &mut Agent, gc: NoGcScope, value: Value) -> JsResult<BigInt> {
    match value {
        // 1. If value is a BigInt, return value.
        Value::BigInt(value) => Ok(value.into()),
        Value::SmallBigInt(value) => Ok(value.into()),
        // 2. If value is an Object and value has a [[BigIntData]] internal slot, then
        Value::PrimitiveObject(value) if value.is_bigint_object(agent) => {
            match agent[value].data {
                // b. Return value.[[BigIntData]].
                PrimitiveObjectData::BigInt(value) => Ok(value.into()),
                PrimitiveObjectData::SmallBigInt(value) => Ok(value.into()),
                // a. Assert: value.[[BigIntData]] is a BigInt.
                _ => unreachable!(),
            }
        }
        // 3. Throw a TypeError exception.
        _ => Err(agent.throw_exception_with_static_message(
            gc,
            ExceptionType::TypeError,
            "Not a BigInt",
        )),
    }
}
