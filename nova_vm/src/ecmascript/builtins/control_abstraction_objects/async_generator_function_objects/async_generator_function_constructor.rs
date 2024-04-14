use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct AsyncGeneratorFunctionConstructor;
impl Builtin for AsyncGeneratorFunctionConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.AsyncGeneratorFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(AsyncGeneratorFunctionConstructor::behaviour);
}

impl AsyncGeneratorFunctionConstructor {
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
        let function_constructor = intrinsics.function();
        let async_generator_function_prototype = intrinsics.async_generator_function_prototype();
        let this = intrinsics.async_generator_function();
        let this_object_index = intrinsics.async_generator_function_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AsyncGeneratorFunctionConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_prototype(function_constructor.into_object())
        .with_property_capacity(1)
        .with_prototype_property(async_generator_function_prototype.into_object())
        .build();
    }
}
