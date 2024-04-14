use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        execution::{Agent, RealmIdentifier},
        types::{IntoValue, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct GeneratorFunctionPrototype;

impl GeneratorFunctionPrototype {
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let function_prototype = intrinsics.function_prototype();
        let generator_prototype = intrinsics.generator_prototype();
        let this = intrinsics.generator_function_prototype();
        let generator_function_constructor = intrinsics.generator_function();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(function_prototype)
            .with_constructor_property(generator_function_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.prototype.into())
                    .with_value_readonly(generator_prototype.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.GeneratorFunction.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
