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

pub(crate) struct DataViewPrototype;

struct DataViewPrototypeGetBuffer;
impl Builtin for DataViewPrototypeGetBuffer {
    const NAME: String = BUILTIN_STRING_MEMORY.get_buffer;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_buffer);
}
struct DataViewPrototypeGetByteLength;
impl Builtin for DataViewPrototypeGetByteLength {
    const NAME: String = BUILTIN_STRING_MEMORY.get_byteLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_byte_length);
}
struct DataViewPrototypeGetByteOffset;
impl Builtin for DataViewPrototypeGetByteOffset {
    const NAME: String = BUILTIN_STRING_MEMORY.get_byteOffset;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_byte_offset);
}
struct DataViewPrototypeGetBigInt64;
impl Builtin for DataViewPrototypeGetBigInt64 {
    const NAME: String = BUILTIN_STRING_MEMORY.getBigInt64;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_big_int64);
}
struct DataViewPrototypeGetBigUint64;
impl Builtin for DataViewPrototypeGetBigUint64 {
    const NAME: String = BUILTIN_STRING_MEMORY.getBigUint64;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_big_uint64);
}
struct DataViewPrototypeGetFloat32;
impl Builtin for DataViewPrototypeGetFloat32 {
    const NAME: String = BUILTIN_STRING_MEMORY.getFloat32;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_float32);
}
struct DataViewPrototypeGetFloat64;
impl Builtin for DataViewPrototypeGetFloat64 {
    const NAME: String = BUILTIN_STRING_MEMORY.getFloat64;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_float64);
}
struct DataViewPrototypeGetInt8;
impl Builtin for DataViewPrototypeGetInt8 {
    const NAME: String = BUILTIN_STRING_MEMORY.getInt8;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_int8);
}
struct DataViewPrototypeGetInt16;
impl Builtin for DataViewPrototypeGetInt16 {
    const NAME: String = BUILTIN_STRING_MEMORY.getInt16;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_int16);
}
struct DataViewPrototypeGetInt32;
impl Builtin for DataViewPrototypeGetInt32 {
    const NAME: String = BUILTIN_STRING_MEMORY.getInt32;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_int32);
}
struct DataViewPrototypeGetUint8;
impl Builtin for DataViewPrototypeGetUint8 {
    const NAME: String = BUILTIN_STRING_MEMORY.getUint8;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_uint8);
}
struct DataViewPrototypeGetUint16;
impl Builtin for DataViewPrototypeGetUint16 {
    const NAME: String = BUILTIN_STRING_MEMORY.getUint16;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_uint16);
}
struct DataViewPrototypeGetUint32;
impl Builtin for DataViewPrototypeGetUint32 {
    const NAME: String = BUILTIN_STRING_MEMORY.getUint32;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_uint32);
}
struct DataViewPrototypeSetBigInt64;
impl Builtin for DataViewPrototypeSetBigInt64 {
    const NAME: String = BUILTIN_STRING_MEMORY.setBigInt64;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_big_int64);
}
struct DataViewPrototypeSetBigUint64;
impl Builtin for DataViewPrototypeSetBigUint64 {
    const NAME: String = BUILTIN_STRING_MEMORY.setBigUint64;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_big_uint64);
}
struct DataViewPrototypeSetFloat32;
impl Builtin for DataViewPrototypeSetFloat32 {
    const NAME: String = BUILTIN_STRING_MEMORY.setFloat32;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_float32);
}
struct DataViewPrototypeSetFloat64;
impl Builtin for DataViewPrototypeSetFloat64 {
    const NAME: String = BUILTIN_STRING_MEMORY.setFloat64;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_float64);
}
struct DataViewPrototypeSetInt8;
impl Builtin for DataViewPrototypeSetInt8 {
    const NAME: String = BUILTIN_STRING_MEMORY.setInt8;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_int8);
}
struct DataViewPrototypeSetInt16;
impl Builtin for DataViewPrototypeSetInt16 {
    const NAME: String = BUILTIN_STRING_MEMORY.setInt16;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_int16);
}
struct DataViewPrototypeSetInt32;
impl Builtin for DataViewPrototypeSetInt32 {
    const NAME: String = BUILTIN_STRING_MEMORY.setInt32;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_int32);
}
struct DataViewPrototypeSetUint8;
impl Builtin for DataViewPrototypeSetUint8 {
    const NAME: String = BUILTIN_STRING_MEMORY.setUint8;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_uint8);
}
struct DataViewPrototypeSetUint16;
impl Builtin for DataViewPrototypeSetUint16 {
    const NAME: String = BUILTIN_STRING_MEMORY.setUint16;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_uint16);
}
struct DataViewPrototypeSetUint32;
impl Builtin for DataViewPrototypeSetUint32 {
    const NAME: String = BUILTIN_STRING_MEMORY.setUint32;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_uint32);
}

impl DataViewPrototype {
    fn get_buffer(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_byte_length(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_byte_offset(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    fn get_big_int64(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_big_uint64(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_float32(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_float64(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_int8(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_int16(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_int32(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_uint8(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_uint16(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn get_uint32(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_big_int64(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_big_uint64(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_float32(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_float64(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_int8(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_int16(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_int32(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_uint8(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_uint16(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn set_uint32(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.data_view_prototype();
        let data_view_constructor = intrinsics.data_view();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(25)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.buffer.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<DataViewPrototypeGetBuffer>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(DataViewPrototypeGetBuffer::ENUMERABLE)
                    .with_configurable(DataViewPrototypeGetBuffer::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.byteLength.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<DataViewPrototypeGetByteLength>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(DataViewPrototypeGetByteLength::ENUMERABLE)
                    .with_configurable(DataViewPrototypeGetByteLength::CONFIGURABLE)
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.byteOffset.into())
                    .with_getter(|agent| {
                        BuiltinFunctionBuilder::new::<DataViewPrototypeGetByteOffset>(agent, realm)
                            .build()
                            .into_function()
                    })
                    .with_enumerable(DataViewPrototypeGetByteOffset::ENUMERABLE)
                    .with_configurable(DataViewPrototypeGetByteOffset::CONFIGURABLE)
                    .build()
            })
            .with_constructor_property(data_view_constructor)
            .with_builtin_function_property::<DataViewPrototypeGetBigInt64>()
            .with_builtin_function_property::<DataViewPrototypeGetBigUint64>()
            .with_builtin_function_property::<DataViewPrototypeGetFloat32>()
            .with_builtin_function_property::<DataViewPrototypeGetFloat64>()
            .with_builtin_function_property::<DataViewPrototypeGetInt8>()
            .with_builtin_function_property::<DataViewPrototypeGetInt16>()
            .with_builtin_function_property::<DataViewPrototypeGetInt32>()
            .with_builtin_function_property::<DataViewPrototypeGetUint8>()
            .with_builtin_function_property::<DataViewPrototypeGetUint16>()
            .with_builtin_function_property::<DataViewPrototypeGetUint32>()
            .with_builtin_function_property::<DataViewPrototypeSetBigInt64>()
            .with_builtin_function_property::<DataViewPrototypeSetBigUint64>()
            .with_builtin_function_property::<DataViewPrototypeSetFloat32>()
            .with_builtin_function_property::<DataViewPrototypeSetFloat64>()
            .with_builtin_function_property::<DataViewPrototypeSetInt8>()
            .with_builtin_function_property::<DataViewPrototypeSetInt16>()
            .with_builtin_function_property::<DataViewPrototypeSetInt32>()
            .with_builtin_function_property::<DataViewPrototypeSetUint8>()
            .with_builtin_function_property::<DataViewPrototypeSetUint16>()
            .with_builtin_function_property::<DataViewPrototypeSetUint32>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.DataView.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
