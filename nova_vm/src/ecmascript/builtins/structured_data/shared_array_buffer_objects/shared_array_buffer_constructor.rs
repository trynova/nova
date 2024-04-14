use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoFunction, IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SharedArrayBufferConstructor;
impl Builtin for SharedArrayBufferConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.SharedArrayBuffer;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(SharedArrayBufferConstructor::behaviour);
}

struct SharedArrayBufferGetSpecies;
impl Builtin for SharedArrayBufferGetSpecies {
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferConstructor::species);
}

impl SharedArrayBufferConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn species(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let shared_array_buffer_prototype = intrinsics.shared_array_buffer_prototype();
        let this = intrinsics.shared_array_buffer();
        let this_object_index = intrinsics.shared_array_buffer_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<SharedArrayBufferConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(2)
        .with_prototype_property(shared_array_buffer_prototype.into_object())
        .with_property(|builder| {
            builder
                .with_key(WellKnownSymbolIndexes::Species.into())
                .with_getter(|agent| {
                    BuiltinFunctionBuilder::new::<SharedArrayBufferGetSpecies>(agent, realm)
                        .build()
                        .into_function()
                })
                .with_enumerable(false)
                .with_configurable(true)
                .build()
        })
        .build();
    }
}
