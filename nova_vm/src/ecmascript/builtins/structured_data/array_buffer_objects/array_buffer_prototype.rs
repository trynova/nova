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

pub(crate) struct ArrayBufferPrototype;

struct ArrayBufferPrototypeGetByteLength;
impl Builtin for ArrayBufferPrototypeGetByteLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_byteLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_byte_length);
}
struct ArrayBufferPrototypeGetDetached;
impl Builtin for ArrayBufferPrototypeGetDetached {
    const NAME: String = BUILTIN_STRING_MEMORY.get_detached;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_detached);
}
struct ArrayBufferPrototypeGetMaxByteLength;
impl Builtin for ArrayBufferPrototypeGetMaxByteLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_maxByteLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_max_byte_length);
}
struct ArrayBufferPrototypeGetResizable;
impl Builtin for ArrayBufferPrototypeGetResizable {
    const NAME: String = BUILTIN_STRING_MEMORY.get_resizable;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_resizable);
}
struct ArrayBufferPrototypeResize;
impl Builtin for ArrayBufferPrototypeResize {
    const NAME: String = BUILTIN_STRING_MEMORY.resize;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::resize);
}
struct ArrayBufferPrototypeSlice;
impl Builtin for ArrayBufferPrototypeSlice {
    const NAME: String = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::slice);
}
struct ArrayBufferPrototypeTransfer;
impl Builtin for ArrayBufferPrototypeTransfer {
    const NAME: String = BUILTIN_STRING_MEMORY.transfer;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::transfer);
}
struct ArrayBufferPrototypeTransferToFixedLength;
impl Builtin for ArrayBufferPrototypeTransferToFixedLength {
    const NAME: String = BUILTIN_STRING_MEMORY.transferToFixedLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::transfer_to_fixed_length);
}

impl ArrayBufferPrototype {
    fn get_byte_length(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_detached(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_max_byte_length(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_resizable(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn resize(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn slice(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn transfer(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn transfer_to_fixed_length(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.array_buffer_prototype();
        let array_buffer_constructor = intrinsics.array_buffer();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(10)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.byteLength.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<ArrayBufferPrototypeGetByteLength>(
                            agent, realm,
                        )
                        .build()
                        .into_function()
                    })
                    .with_enumerable(ArrayBufferPrototypeGetByteLength::ENUMERABLE)
                    .with_configurable(ArrayBufferPrototypeGetByteLength::CONFIGURABLE)
                    .build()
            })
            .with_constructor_property(array_buffer_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.detached.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<ArrayBufferPrototypeGetDetached>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(ArrayBufferPrototypeGetDetached::ENUMERABLE)
                    .with_configurable(ArrayBufferPrototypeGetDetached::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.maxByteLength.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<ArrayBufferPrototypeGetMaxByteLength>(
                            agent, realm,
                        )
                        .build()
                        .into_function()
                    })
                    .with_enumerable(ArrayBufferPrototypeGetMaxByteLength::ENUMERABLE)
                    .with_configurable(ArrayBufferPrototypeGetMaxByteLength::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.resizable.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<ArrayBufferPrototypeGetResizable>(
                            agent, realm,
                        )
                        .build()
                        .into_function()
                    })
                    .with_enumerable(ArrayBufferPrototypeGetResizable::ENUMERABLE)
                    .with_configurable(ArrayBufferPrototypeGetResizable::CONFIGURABLE)
                    .build()
            })
            .with_builtin_function_property::<ArrayBufferPrototypeResize>()
            .with_builtin_function_property::<ArrayBufferPrototypeSlice>()
            .with_builtin_function_property::<ArrayBufferPrototypeTransfer>()
            .with_builtin_function_property::<ArrayBufferPrototypeTransferToFixedLength>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.ArrayBuffer.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
