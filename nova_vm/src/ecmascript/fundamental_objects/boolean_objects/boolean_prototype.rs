use crate::ecmascript::{
    builders::ordinary_object_builder::OrdinaryObjectBuilder,
    builtins::{ArgumentsList, Builtin},
    execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
    types::{String, Value},
};

pub(crate) struct BooleanPrototype;

struct BooleanPrototypeToString;
impl Builtin for BooleanPrototypeToString {
    const NAME: &'static str = "toString";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BooleanPrototype::to_string);
}

struct BooleanPrototypeValueOf;
impl Builtin for BooleanPrototypeValueOf {
    const NAME: &'static str = "valueOf";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BooleanPrototype::value_of);
}

impl BooleanPrototype {
    fn to_string(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        let b = this_boolean_value(agent, this_value)?;
        if b {
            Ok(String::from_small_string("true").into_value())
        } else {
            Ok(String::from_small_string("false").into_value())
        }
    }

    fn value_of(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        this_boolean_value(agent, this_value).map(|result| result.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.boolean_prototype();
        let boolean_constructor = intrinsics.boolean();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_constructor_property(boolean_constructor)
            .with_builtin_function_property::<BooleanPrototypeToString>()
            .with_builtin_function_property::<BooleanPrototypeValueOf>()
            .build();
    }
}

///  ### [20.3.3.3.1 ThisBooleanValue ( value )](https://tc39.es/ecma262/#sec-thisbooleanvalue)
///
/// The abstract operation ThisBooleanValue takes argument value (an
/// ECMAScript language value) and returns either a normal completion
/// containing a Boolean or a throw completion.
fn this_boolean_value(agent: &mut Agent, value: Value) -> JsResult<bool> {
    // 1. If value is a Boolean, return value.
    if let Value::Boolean(value) = value {
        return Ok(value);
    }
    // TODO: Boolean objects
    // 2. If value is an Object and value has a [[BooleanData]] internal slot, then
    // a. Let b be value.[[BooleanData]].
    // b. Assert: b is a Boolean.
    // c. Return b.
    // 3. Throw a TypeError exception.
    Err(agent.throw_exception(ExceptionType::TypeError, "Not a Boolean or Boolean object"))
}
