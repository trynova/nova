use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct ArrayBufferConstructor;
impl Builtin for ArrayBufferConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.ArrayBuffer;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(ArrayBufferConstructor::behaviour);
}

struct ArrayBufferIsView;
impl Builtin for ArrayBufferIsView {
    const NAME: String = BUILTIN_STRING_MEMORY.isView;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferConstructor::is_view);
}

struct ArrayBufferGetSpecies;
impl Builtin for ArrayBufferGetSpecies {
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferConstructor::species);
}
impl BuiltinGetter for ArrayBufferGetSpecies {
    const KEY: PropertyKey = WellKnownSymbolIndexes::Species.to_property_key();
}

impl ArrayBufferConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn is_view(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
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
        let array_buffer_prototype = intrinsics.array_buffer_prototype();
        let this = intrinsics.array_buffer();
        let this_object_index = intrinsics.array_buffer_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ArrayBufferConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(3)
        .with_builtin_function_property::<ArrayBufferIsView>()
        .with_prototype_property(array_buffer_prototype.into_object())
        .with_builtin_function_getter_property::<ArrayBufferGetSpecies>()
        .build();
    }
}
