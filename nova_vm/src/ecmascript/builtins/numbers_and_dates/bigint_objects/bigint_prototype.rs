use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Builtin},
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{BigInt, IntoValue, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct BigIntPrototype;

struct BigIntPrototypeToLocaleString;
impl Builtin for BigIntPrototypeToLocaleString {
    const NAME: String = BUILTIN_STRING_MEMORY.toLocaleString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BigIntPrototype::to_locale_string);
}

struct BigIntPrototypeToString;
impl Builtin for BigIntPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BigIntPrototype::to_string);
}

struct BigIntPrototypeValueOf;
impl Builtin for BigIntPrototypeValueOf {
    const NAME: String = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BigIntPrototype::value_of);
}

impl BigIntPrototype {
    fn to_locale_string(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Self::to_string(agent, this_value, arguments)
    }

    fn to_string(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let _x = this_big_int_value(agent, this_value)?;
        let radix = arguments.get(0);
        if radix.is_undefined() || radix == Value::from(10u8) {
            // BigInt::to_string_radix_10(agent, x).map(|result| result.into_value())
            todo!();
        } else {
            todo!();
        }
    }

    fn value_of(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        this_big_int_value(agent, this_value).map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.big_int_prototype();
        let big_int_constructor = intrinsics.big_int();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_constructor_property(big_int_constructor)
            .with_builtin_function_property::<BigIntPrototypeToLocaleString>()
            .with_builtin_function_property::<BigIntPrototypeToString>()
            .with_builtin_function_property::<BigIntPrototypeValueOf>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BigInt.into())
                    .with_value_readonly(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

fn this_big_int_value(agent: &mut Agent, value: Value) -> JsResult<BigInt> {
    match value {
        Value::BigInt(idx) => Ok(idx.into()),
        Value::SmallBigInt(data) => Ok(data.into()),
        // TODO: Primitive objects
        _ => Err(agent.throw_exception(ExceptionType::TypeError, "Not a BigInt")),
    }
}
