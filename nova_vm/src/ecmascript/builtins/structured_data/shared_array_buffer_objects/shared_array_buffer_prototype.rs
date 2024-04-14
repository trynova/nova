use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoFunction, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SharedArrayBufferPrototype;

struct SharedArrayBufferPrototypeGetByteLength;
impl Builtin for SharedArrayBufferPrototypeGetByteLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_byteLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::get_byte_length);
}
struct SharedArrayBufferPrototypeGrow;
impl Builtin for SharedArrayBufferPrototypeGrow {
    const NAME: String = BUILTIN_STRING_MEMORY.grow;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::grow);
}
struct SharedArrayBufferPrototypeGetGrowable;
impl Builtin for SharedArrayBufferPrototypeGetGrowable {
    const NAME: String = BUILTIN_STRING_MEMORY.get_growable;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::get_growable);
}
struct SharedArrayBufferPrototypeGetMaxByteLength;
impl Builtin for SharedArrayBufferPrototypeGetMaxByteLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_maxByteLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(SharedArrayBufferPrototype::get_max_byte_length);
}
struct SharedArrayBufferPrototypeSlice;
impl Builtin for SharedArrayBufferPrototypeSlice {
    const NAME: String = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::slice);
}

impl SharedArrayBufferPrototype {
    fn get_byte_length(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn grow(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_growable(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_max_byte_length(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn slice(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.shared_array_buffer_prototype();
        let shared_array_buffer_constructor = intrinsics.shared_array_buffer();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(7)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.byteLength.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<SharedArrayBufferPrototypeGetByteLength>(
                            agent, realm,
                        )
                        .build()
                        .into_function()
                    })
                    .with_enumerable(SharedArrayBufferPrototypeGetByteLength::ENUMERABLE)
                    .with_configurable(SharedArrayBufferPrototypeGetByteLength::CONFIGURABLE)
                    .build()
            })
            .with_constructor_property(shared_array_buffer_constructor)
            .with_builtin_function_property::<SharedArrayBufferPrototypeGrow>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.growable.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<SharedArrayBufferPrototypeGetGrowable>(
                            agent, realm,
                        )
                        .build()
                        .into_function()
                    })
                    .with_enumerable(SharedArrayBufferPrototypeGetGrowable::ENUMERABLE)
                    .with_configurable(SharedArrayBufferPrototypeGetGrowable::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.maxByteLength.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<SharedArrayBufferPrototypeGetMaxByteLength>(
                            agent, realm,
                        )
                        .build()
                        .into_function()
                    })
                    .with_enumerable(SharedArrayBufferPrototypeGetMaxByteLength::ENUMERABLE)
                    .with_configurable(SharedArrayBufferPrototypeGetMaxByteLength::CONFIGURABLE)
                    .build()
            })
            .with_builtin_function_property::<SharedArrayBufferPrototypeSlice>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.SharedArrayBuffer.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
