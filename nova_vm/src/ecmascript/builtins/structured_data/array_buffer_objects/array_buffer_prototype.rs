use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
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
impl BuiltinGetter for ArrayBufferPrototypeGetByteLength {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.byteLength.to_property_key();
}
struct ArrayBufferPrototypeGetDetached;
impl Builtin for ArrayBufferPrototypeGetDetached {
    const NAME: String = BUILTIN_STRING_MEMORY.get_detached;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_detached);
}
impl BuiltinGetter for ArrayBufferPrototypeGetDetached {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.detached.to_property_key();
}
struct ArrayBufferPrototypeGetMaxByteLength;
impl Builtin for ArrayBufferPrototypeGetMaxByteLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_maxByteLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_max_byte_length);
}
impl BuiltinGetter for ArrayBufferPrototypeGetMaxByteLength {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.maxByteLength.to_property_key();
}
struct ArrayBufferPrototypeGetResizable;
impl Builtin for ArrayBufferPrototypeGetResizable {
    const NAME: String = BUILTIN_STRING_MEMORY.get_resizable;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_resizable);
}
impl BuiltinGetter for ArrayBufferPrototypeGetResizable {
    const KEY: PropertyKey = BUILTIN_STRING_MEMORY.resizable.to_property_key();
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
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.array_buffer_prototype();
        let array_buffer_constructor = intrinsics.array_buffer();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(10)
            .with_prototype(object_prototype)
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetByteLength>()
            .with_constructor_property(array_buffer_constructor)
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetDetached>()
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetMaxByteLength>()
            .with_builtin_function_getter_property::<ArrayBufferPrototypeGetResizable>()
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
