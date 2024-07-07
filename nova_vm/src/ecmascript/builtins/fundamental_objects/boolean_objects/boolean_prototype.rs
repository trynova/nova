use crate::ecmascript::{
    builders::ordinary_object_builder::OrdinaryObjectBuilder,
    builtins::{
        primitive_objects::{PrimitiveObjectData, PrimitiveObjectHeapData},
        ArgumentsList, Builtin,
    },
    execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
    types::{String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct BooleanPrototype;

struct BooleanPrototypeToString;
impl Builtin for BooleanPrototypeToString {
    const NAME: String = BUILTIN_STRING_MEMORY.toString;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BooleanPrototype::to_string);
}

struct BooleanPrototypeValueOf;
impl Builtin for BooleanPrototypeValueOf {
    const NAME: String = BUILTIN_STRING_MEMORY.valueOf;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(BooleanPrototype::value_of);
}

impl BooleanPrototype {
    fn to_string(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        let b = this_boolean_value(agent, this_value)?;
        if b {
            Ok(BUILTIN_STRING_MEMORY.r#true.into())
        } else {
            Ok(BUILTIN_STRING_MEMORY.r#false.into())
        }
    }

    fn value_of(agent: &mut Agent, this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        this_boolean_value(agent, this_value).map(|result| result.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.boolean_prototype();
        let this_base_object = intrinsics.boolean_prototype_base_object().into();
        let boolean_constructor = intrinsics.boolean();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this_base_object)
            .with_property_capacity(3)
            .with_prototype(object_prototype)
            .with_constructor_property(boolean_constructor)
            .with_builtin_function_property::<BooleanPrototypeToString>()
            .with_builtin_function_property::<BooleanPrototypeValueOf>()
            .build();

        let slot = agent
            .heap
            .primitive_objects
            .get_mut(this.get_index())
            .unwrap();
        assert!(slot.is_none());
        *slot = Some(PrimitiveObjectHeapData {
            object_index: Some(this_base_object),
            data: PrimitiveObjectData::Boolean(false),
        });
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
    } else if let Value::PrimitiveObject(value) = value {
        // 2. If value is an Object and value has a [[BooleanData]] internal slot, then
        if let PrimitiveObjectData::Boolean(b) = agent[value].data {
            // a. Let b be value.[[BooleanData]].
            // b. Assert: b is a Boolean.
            // c. Return b.
            return Ok(b);
        }
    }
    // 3. Throw a TypeError exception.
    Err(agent.throw_exception(ExceptionType::TypeError, "Not a Boolean or Boolean object"))
}
