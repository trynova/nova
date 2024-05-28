use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ordinary::get_prototype_from_constructor;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Symbol;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::heap::IntrinsicConstructorIndexes;

pub struct StringConstructor;

impl Builtin for StringConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.String;
}
impl BuiltinIntrinsicConstructor for StringConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::String;
}

struct StringFromCharCode;
impl Builtin for StringFromCharCode {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringConstructor::from_char_code);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.fromCharCode;
}
struct StringFromCodePoint;
impl Builtin for StringFromCodePoint {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringConstructor::from_code_point);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.fromCodePoint;
}
struct StringRaw;
impl Builtin for StringRaw {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringConstructor::raw);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.raw;
}
impl StringConstructor {
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let value = arguments.get(0);
        // 1. If value is not present, then
        let s = if value.is_undefined() {
            // a. Let s be the empty String.
            String::EMPTY_STRING
        } else {
            // 2. Else,
            // a. If NewTarget is undefined and value is a Symbol, return SymbolDescriptiveString(value).
            if new_target.is_none() {
                if let Value::Symbol(value) = value {
                    return Ok(Symbol::from(value).descriptive_string(agent).into_value());
                }
            }
            // b. Let s be ? ToString(value).
            to_string(agent, value)?
        };
        // 3. If NewTarget is undefined, return s.
        let Some(new_target) = new_target else {
            return Ok(s.into_value());
        };
        // 4. Return StringCreate(s, ? GetPrototypeFromConstructor(NewTarget, "%String.prototype%")).
        let _prototype = get_prototype_from_constructor(
            agent,
            Function::try_from(new_target).unwrap(),
            ProtoIntrinsics::String,
        )?;
        todo!("StringCreate(s, ? GetPrototypeFromConstructor(NewTarget, \"%String.prototype%\"))");
    }

    fn from_char_code(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn from_code_point(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn raw(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let string_prototype = intrinsics.string_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<StringConstructor>(agent, realm)
            .with_property_capacity(4)
            .with_builtin_function_property::<StringFromCharCode>()
            .with_builtin_function_property::<StringFromCodePoint>()
            .with_prototype_property(string_prototype.into_object())
            .with_builtin_function_property::<StringRaw>()
            .build();
    }
}
