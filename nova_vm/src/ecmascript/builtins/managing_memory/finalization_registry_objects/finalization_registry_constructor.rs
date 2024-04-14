use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct FinalizationRegistryConstructor;
impl Builtin for FinalizationRegistryConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(FinalizationRegistryConstructor::behaviour);
}

impl FinalizationRegistryConstructor {
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
        let finalization_registry_prototype = intrinsics.finalization_registry_prototype();
        let this = intrinsics.finalization_registry();
        let this_object_index = intrinsics.finalization_registry_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<FinalizationRegistryConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(1)
        .with_prototype_property(finalization_registry_prototype.into_object())
        .build();
    }
}
