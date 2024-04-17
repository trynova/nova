use crate::ecmascript::abstract_operations::testing_and_comparison::is_array;

use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;

use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinGetter;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;

use crate::ecmascript::execution::RealmIdentifier;

use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::PropertyKey;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::heap::WellKnownSymbolIndexes;

pub struct ArrayConstructor;

impl Builtin for ArrayConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.String;
}

struct ArrayFrom;
impl Builtin for ArrayFrom {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::from);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.from;
}
struct ArrayIsArray;
impl Builtin for ArrayIsArray {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::is_array);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.isArray;
}
struct ArrayOf;
impl Builtin for ArrayOf {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::of);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.fromCodePoint;
}
struct ArrayGetSpecies;
impl Builtin for ArrayGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl BuiltinGetter for ArrayGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}

impl ArrayConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        // 1. If NewTarget is undefined, let newTarget be the active function object; else let newTarget be NewTarget.
        // 2. Let proto be ? GetPrototypeFromConstructor(newTarget, "%Array.prototype%").
        // 3. Let numberOfArgs be the number of elements in values.
        // 4. If numberOfArgs = 0, then
        // a. Return ! ArrayCreate(0, proto).
        // 5. Else if numberOfArgs = 1, then
        // a. Let len be values[0].
        // b. Let array be ! ArrayCreate(0, proto).
        // c. If len is not a Number, then
        // i. Perform ! CreateDataPropertyOrThrow(array, "0", len).
        // ii. Let intLen be 1𝔽.
        // d. Else,
        // i. Let intLen be ! ToUint32(len).
        // ii. If SameValueZero(intLen, len) is false, throw a RangeError exception.
        // e. Perform ! Set(array, "length", intLen, true).
        // f. Return array.
        // 6. Else,
        // a. Assert: numberOfArgs ≥ 2.
        // b. Let array be ? ArrayCreate(numberOfArgs, proto).
        // c. Let k be 0.
        // d. Repeat, while k < numberOfArgs,
        // i. Let Pk be ! ToString(𝔽(k)).
        // ii. Let itemK be values[k].
        // iii. Perform ! CreateDataPropertyOrThrow(array, Pk, itemK).
        // iv. Set k to k + 1.
        // e. Assert: The mathematical value of array's "length" property is numberOfArgs.
        // f. Return array.
        todo!()
    }

    fn from(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn is_array(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        is_array(agent, arguments.get(0)).map(Value::Boolean)
    }

    fn of(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn get_species(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let array_prototype = intrinsics.array_prototype();
        let this = intrinsics.array();
        let this_object_index = intrinsics.array_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ArrayConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(5)
        .with_builtin_function_property::<ArrayFrom>()
        .with_builtin_function_property::<ArrayIsArray>()
        .with_builtin_function_property::<ArrayOf>()
        .with_prototype_property(array_prototype.into_object())
        .with_builtin_function_getter_property::<ArrayGetSpecies>()
        .build();
    }
}
