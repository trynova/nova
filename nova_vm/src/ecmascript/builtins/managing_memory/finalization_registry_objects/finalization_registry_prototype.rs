use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct FinalizationRegistryPrototype;

struct FinalizationRegistryPrototypeRegister;
impl Builtin for FinalizationRegistryPrototypeRegister {
    const NAME: String = BUILTIN_STRING_MEMORY.every;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(FinalizationRegistryPrototype::register);
}
struct FinalizationRegistryPrototypeUnregister;
impl Builtin for FinalizationRegistryPrototypeUnregister {
    const NAME: String = BUILTIN_STRING_MEMORY.get;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(FinalizationRegistryPrototype::unregister);
}

impl FinalizationRegistryPrototype {
    fn register(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn unregister(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.finalization_registry_prototype();
        let finalization_registry_constructor = intrinsics.finalization_registry();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(4)
            .with_constructor_property(finalization_registry_constructor)
            .with_builtin_function_property::<FinalizationRegistryPrototypeRegister>()
            .with_builtin_function_property::<FinalizationRegistryPrototypeUnregister>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.FinalizationRegistry.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
