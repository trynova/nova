// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_integer_or_infinity,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, primitive_objects::PrimitiveObjectData},
        execution::{Agent, JsResult, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, BigInt, String, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct BigIntPrototype;

struct BigIntPrototypeToLocaleString;
impl Builtin for BigIntPrototypeToLocaleString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toLocaleString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(BigIntPrototype::to_locale_string);
}

struct BigIntPrototypeToString;
impl Builtin for BigIntPrototypeToString {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(BigIntPrototype::to_string);
}

struct BigIntPrototypeValueOf;
impl Builtin for BigIntPrototypeValueOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(BigIntPrototype::value_of);
}

impl BigIntPrototype {
    /// ### [21.2.3.2 BigInt.prototype.toLocaleString ( \[ reserved1 \[ , reserved2 \] \] )](https://tc39.es/ecma262/#sec-bigint.prototype.tolocalestring)
    fn to_locale_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::to_string(agent, this_value, arguments, gc)
    }

    /// ### [21.2.3.3 BigInt.prototype.toString ( [ radix ] )](https://tc39.es/ecma262/#sec-bigint.prototype.tostring)
    ///
    /// > NOTE: The optional radix should be an integral Number value in the
    /// > inclusive interval from 2𝔽 to 36𝔽. If radix is undefined then 10𝔽 is
    /// > used as the value of radix.
    fn to_string<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let x = this_big_int_value(agent, this_value, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());
        let radix = arguments.get(0).bind(gc.nogc());
        // 2. If radix is undefined, let radixMV be 10.
        if radix.is_undefined() || radix == Value::from(10u8) {
            // 5. Return BigInt::toString(x, 10).
            Ok(BigInt::to_string_radix_10(agent, x.get(agent), gc.nogc())
                .unbind()
                .into())
        } else {
            // 3. Else, let radixMV be ? ToIntegerOrInfinity(radix).
            let radix = to_integer_or_infinity(agent, radix.unbind(), gc.reborrow()).unbind()?;
            let gc = gc.into_nogc();
            let radix = radix.bind(gc);
            // 4. If radixMV is not in the inclusive interval from 2 to 36, throw a RangeError exception.
            if !(2..=36).contains(&radix) {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "radix must be an integer at least 2 and no greater than 36",
                    gc,
                ));
            }
            let radix = radix.into_i64() as u32;
            // 5. Return BigInt::toString(x, radixMV).
            Ok(BigInt::to_string_radix_n(agent, x.get(agent), radix, gc).into())
        }
    }

    /// ### [21.2.3.4 BigInt.prototype.valueOf ( )](https://tc39.es/ecma262/#sec-bigint.prototype.valueof)
    fn value_of<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return ? ThisBigIntValue(this value).
        this_big_int_value(agent, this_value, gc.into_nogc()).map(|result| result.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
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
fn this_big_int_value<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, BigInt<'a>> {
    match value {
        // 1. If value is a BigInt, return value.
        Value::BigInt(value) => Ok(value.unbind().into()),
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
            ExceptionType::TypeError,
            "Not a BigInt",
            gc,
        )),
    }
}
