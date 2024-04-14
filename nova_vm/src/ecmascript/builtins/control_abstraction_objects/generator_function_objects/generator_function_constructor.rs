use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct GeneratorFunctionConstructor;
impl Builtin for GeneratorFunctionConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.GeneratorFunction;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(GeneratorFunctionConstructor::behaviour);
}

impl GeneratorFunctionConstructor {
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
        let generator_function_prototype = intrinsics.generator_function_prototype();
        let this = intrinsics.generator_function();
        let this_object_index = intrinsics.generator_function_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<GeneratorFunctionConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(1)
        .with_prototype_property(generator_function_prototype.into_object())
        .build();
    }
}
