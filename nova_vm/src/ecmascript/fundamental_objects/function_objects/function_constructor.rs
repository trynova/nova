use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::Value;

pub(crate) struct FunctionConstructor;

impl Builtin for FunctionConstructor {
    const NAME: &'static str = "Function";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}

impl FunctionConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.function();
        let this_object_index = intrinsics.function_base_object();
        let function_prototype = intrinsics.function_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<FunctionConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(1)
        .with_property(|builder| {
            builder
                .with_key_from_str("prototype")
                .with_value_readonly(function_prototype.into_value())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .build();
    }
}
