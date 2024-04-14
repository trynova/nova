use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct AsyncFunctionConstructor;
impl Builtin for AsyncFunctionConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.AsyncFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(AsyncFunctionConstructor::behaviour);
}

impl AsyncFunctionConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let async_function_prototype = intrinsics.async_function_prototype();
        let function_constructor = intrinsics.function();
        let this = intrinsics.async_function();
        let this_object_index = intrinsics.async_function_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AsyncFunctionConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_prototype(function_constructor.into_object())
        .with_property_capacity(1)
        .with_prototype_property(async_function_prototype.into_object())
        .build();
    }
}
