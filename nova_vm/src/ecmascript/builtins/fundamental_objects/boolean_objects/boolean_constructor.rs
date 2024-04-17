use crate::ecmascript::abstract_operations::type_conversion::to_boolean;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ordinary::ordinary_create_from_constructor;
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
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::{String, Value};
use crate::heap::IntrinsicConstructorIndexes;

pub(crate) struct BooleanConstructor;

impl Builtin for BooleanConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Boolean;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}
impl BuiltinIntrinsicConstructor for BooleanConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Boolean;
}

impl BooleanConstructor {
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let value = arguments.get(0);
        let b = to_boolean(agent, value);
        let Some(new_target) = new_target else {
            return Ok(b.into());
        };
        let new_target = Function::try_from(new_target).unwrap();
        let _ = ordinary_create_from_constructor(agent, new_target, ProtoIntrinsics::Boolean, ())?;
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let boolean_prototype = intrinsics.boolean_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<BooleanConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(boolean_prototype.into_object())
            .build();
    }
}
