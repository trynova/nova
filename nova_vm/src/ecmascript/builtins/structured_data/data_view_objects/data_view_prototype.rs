// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct DataViewPrototype;

struct DataViewPrototypeGetBuffer;
impl Builtin for DataViewPrototypeGetBuffer {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_buffer;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.buffer.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_buffer);
}
impl BuiltinGetter for DataViewPrototypeGetBuffer {}
struct DataViewPrototypeGetByteLength;
impl Builtin for DataViewPrototypeGetByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteLength;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.byteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_byte_length);
}
impl BuiltinGetter for DataViewPrototypeGetByteLength {}
struct DataViewPrototypeGetByteOffset;
impl Builtin for DataViewPrototypeGetByteOffset {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteOffset;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.byteOffset.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_byte_offset);
}
impl BuiltinGetter for DataViewPrototypeGetByteOffset {}
struct DataViewPrototypeGetBigInt64;
impl Builtin for DataViewPrototypeGetBigInt64 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getBigInt64;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_big_int64);
}
struct DataViewPrototypeGetBigUint64;
impl Builtin for DataViewPrototypeGetBigUint64 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getBigUint64;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_big_uint64);
}
struct DataViewPrototypeGetFloat32;
impl Builtin for DataViewPrototypeGetFloat32 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getFloat32;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_float32);
}
struct DataViewPrototypeGetFloat64;
impl Builtin for DataViewPrototypeGetFloat64 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getFloat64;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_float64);
}
struct DataViewPrototypeGetInt8;
impl Builtin for DataViewPrototypeGetInt8 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getInt8;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_int8);
}
struct DataViewPrototypeGetInt16;
impl Builtin for DataViewPrototypeGetInt16 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getInt16;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_int16);
}
struct DataViewPrototypeGetInt32;
impl Builtin for DataViewPrototypeGetInt32 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getInt32;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_int32);
}
struct DataViewPrototypeGetUint8;
impl Builtin for DataViewPrototypeGetUint8 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUint8;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_uint8);
}
struct DataViewPrototypeGetUint16;
impl Builtin for DataViewPrototypeGetUint16 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUint16;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_uint16);
}
struct DataViewPrototypeGetUint32;
impl Builtin for DataViewPrototypeGetUint32 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getUint32;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_uint32);
}
struct DataViewPrototypeSetBigInt64;
impl Builtin for DataViewPrototypeSetBigInt64 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setBigInt64;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_big_int64);
}
struct DataViewPrototypeSetBigUint64;
impl Builtin for DataViewPrototypeSetBigUint64 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setBigUint64;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_big_uint64);
}
struct DataViewPrototypeSetFloat32;
impl Builtin for DataViewPrototypeSetFloat32 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setFloat32;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_float32);
}
struct DataViewPrototypeSetFloat64;
impl Builtin for DataViewPrototypeSetFloat64 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setFloat64;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_float64);
}
struct DataViewPrototypeSetInt8;
impl Builtin for DataViewPrototypeSetInt8 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setInt8;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_int8);
}
struct DataViewPrototypeSetInt16;
impl Builtin for DataViewPrototypeSetInt16 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setInt16;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_int16);
}
struct DataViewPrototypeSetInt32;
impl Builtin for DataViewPrototypeSetInt32 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setInt32;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_int32);
}
struct DataViewPrototypeSetUint8;
impl Builtin for DataViewPrototypeSetUint8 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUint8;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_uint8);
}
struct DataViewPrototypeSetUint16;
impl Builtin for DataViewPrototypeSetUint16 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUint16;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_uint16);
}
struct DataViewPrototypeSetUint32;
impl Builtin for DataViewPrototypeSetUint32 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setUint32;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_uint32);
}

impl DataViewPrototype {
    fn get_buffer<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_byte_length<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_byte_offset<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_big_int64<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_big_uint64<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_float32<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_float64<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_int8<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_int16<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_int32<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_uint8<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_uint16<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_uint32<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_big_int64<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_big_uint64<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_float32<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_float64<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_int8<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_int16<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_int32<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_uint8<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_uint16<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn set_uint32<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.data_view_prototype();
        let data_view_constructor = intrinsics.data_view();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(25)
            .with_prototype(object_prototype)
            .with_builtin_function_getter_property::<DataViewPrototypeGetBuffer>()
            .with_builtin_function_getter_property::<DataViewPrototypeGetByteLength>()
            .with_builtin_function_getter_property::<DataViewPrototypeGetByteOffset>()
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
