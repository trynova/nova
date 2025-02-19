// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::type_conversion::to_boolean;
use crate::ecmascript::builtins::data_view::abstract_operations::{get_view_value, set_view_value};
use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            array_buffer::Ordering,
            data_view::{
                abstract_operations::{
                    get_view_byte_length, is_view_out_of_bounds,
                    make_data_view_with_buffer_witness_record,
                },
                DataView,
            },
            ArgumentsList, Behaviour, Builtin, BuiltinGetter,
        },
        execution::{agent::ExceptionType, Agent, JsResult, RealmIdentifier},
        types::{IntoValue, Number, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
    SmallInteger,
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
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.byteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_byte_length);
}
impl BuiltinGetter for DataViewPrototypeGetByteLength {}
struct DataViewPrototypeGetByteOffset;
impl Builtin for DataViewPrototypeGetByteOffset {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteOffset;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.byteOffset.to_property_key());
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
#[cfg(feature = "proposal-float16array")]
struct DataViewPrototypeGetFloat16;
#[cfg(feature = "proposal-float16array")]
impl Builtin for DataViewPrototypeGetFloat16 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getFloat16;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::get_float16);
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
#[cfg(feature = "proposal-float16array")]
struct DataViewPrototypeSetFloat16;
#[cfg(feature = "proposal-float16array")]
impl Builtin for DataViewPrototypeSetFloat16 {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setFloat16;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(DataViewPrototype::set_float16);
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
    /// ### [25.3.4.1 get DataView.prototype.buffer](https://tc39.es/ecma262/#sec-get-dataview.prototype.buffer)
    ///
    /// DataView.prototype.buffer is an accessor property whose set accessor
    /// function is undefined.
    fn get_buffer<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[DataView]]).
        let gc = gc.into_nogc();
        let o = require_internal_slot_data_view(agent, this_value, gc)?;
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        // 4. Let buffer be O.[[ViewedArrayBuffer]].
        // 5. Return buffer.
        Ok(agent[o].viewed_array_buffer.into_value())
    }

    /// ### [25.3.4.2 get DataView.prototype.byteLength](https://tc39.es/ecma262/#sec-get-dataview.prototype.bytelength)
    ///
    /// DataView.prototype.byteLength is an accessor property whose set accessor
    /// function is undefined.
    fn get_byte_length<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[DataView]]).
        let gc = gc.into_nogc();
        let o = require_internal_slot_data_view(agent, this_value, gc)?;
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        // 4. Let viewRecord be MakeDataViewWithBufferWitnessRecord(O, seq-cst).
        let view_record = make_data_view_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc);
        // 5. If IsViewOutOfBounds(viewRecord) is true, throw a TypeError exception.
        if is_view_out_of_bounds(agent, &view_record, gc) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "DataView is out of bounds",
                gc,
            ));
        }
        // 6. Let size be GetViewByteLength(viewRecord).
        let size = get_view_byte_length(agent, &view_record, gc) as i64;
        // 7. Return 𝔽(size).
        Ok(Number::from(SmallInteger::try_from(size).unwrap()).into_value())
    }

    /// ### [25.3.4.3 get DataView.prototype.byteOffset](https://tc39.es/ecma262/#sec-get-dataview.prototype.byteoffset)
    ///
    /// DataView.prototype.byteOffset is an accessor property whose set accessor
    /// function is undefined.
    fn get_byte_offset<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[DataView]]).
        let gc = gc.into_nogc();
        let o = require_internal_slot_data_view(agent, this_value, gc)?;
        // 3. Assert: O has a [[ViewedArrayBuffer]] internal slot.
        // 4. Let viewRecord be MakeDataViewWithBufferWitnessRecord(O, seq-cst).
        let view_record = make_data_view_with_buffer_witness_record(agent, o, Ordering::SeqCst, gc);
        // 5. If IsViewOutOfBounds(viewRecord) is true, throw a TypeError exception.
        if is_view_out_of_bounds(agent, &view_record, gc) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "DataView is out of bounds",
                gc,
            ));
        }
        // 6. Let offset be O.[[ByteOffset]].
        // 7. Return 𝔽(offset).
        Ok(Number::from(SmallInteger::try_from(o.byte_offset(agent) as i64).unwrap()).into_value())
    }

    /// ### [25.3.4.5 DataView.prototype.getBigInt64 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getbigint64)
    fn get_big_int64<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 2. Return ? GetViewValue(v, byteOffset, littleEndian, bigint64).
        get_view_value::<i64>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.6 DataView.prototype.getBigUint64 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getbiguint64)
    fn get_big_uint64<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 2. Return ? GetViewValue(v, byteOffset, littleEndian, biguint64).
        get_view_value::<u64>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [7.1 DataView.prototype.getFloat16 ( byteOffset [ , littleEndian ] )](https://tc39.es/proposal-float16array/#sec-dataview.prototype.getfloat16)
    #[cfg(feature = "proposal-float16array")]
    fn get_float16<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope,
    ) -> JsResult<Value> {
        let byte_offset = arguments.get(0);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 3. Return ? GetViewValue(v, byteOffset, littleEndian, float16).
        get_view_value::<f16>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.7 DataView.prototype.getFloat32 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getfloat32)
    fn get_float32<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 3. Return ? GetViewValue(v, byteOffset, littleEndian, float32).
        get_view_value::<f32>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.8 DataView.prototype.getFloat64 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getfloat64)
    fn get_float64<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 3. Return ? GetViewValue(v, byteOffset, littleEndian, float64).
        get_view_value::<f64>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.9 DataView.prototype.getInt8 ( byteOffset )](https://tc39.es/ecma262/#sec-dataview.prototype.getint8)
    fn get_int8<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 1. Let v be the this value.
        // 2. Return ? GetViewValue(v, byteOffset, true, int8).
        get_view_value::<i8>(agent, this_value, byte_offset, true, gc).map(IntoValue::into_value)
    }

    /// ### [25.3.4.10 DataView.prototype.getInt16 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getint16)
    fn get_int16<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 3. Return ? GetViewValue(v, byteOffset, littleEndian, int16).
        get_view_value::<i16>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.11 DataView.prototype.getInt32 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getint32)
    fn get_int32<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 3. Return ? GetViewValue(v, byteOffset, littleEndian, int32).
        get_view_value::<i32>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.12 DataView.prototype.getUint8 ( byteOffset )](https://tc39.es/ecma262/#sec-dataview.prototype.getuint8)
    fn get_uint8<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 1. Let v be the this value.
        // 2. Return ? GetViewValue(v, byteOffset, true, uint8).
        get_view_value::<u8>(agent, this_value, byte_offset, true, gc).map(IntoValue::into_value)
    }

    /// ### [25.3.4.13 DataView.prototype.getUint16 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getuint16)
    fn get_uint16<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 3. Return ? GetViewValue(v, byteOffset, littleEndian, uint16).
        get_view_value::<u16>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.14 DataView.prototype.getUint32 ( byteOffset \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.getuint32)
    fn get_uint32<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(1));
        // 1. Let v be the this value.
        // 3. Return ? GetViewValue(v, byteOffset, littleEndian, uint32).
        get_view_value::<u32>(agent, this_value, byte_offset, little_endian, gc)
            .map(IntoValue::into_value)
    }

    /// ### [25.3.4.15 DataView.prototype.setBigInt64 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setbigint64)
    fn set_big_int64<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, bigint64, value).
        set_view_value::<i64>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [25.3.4.16 DataView.prototype.setBigUint64 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setbiguint64)
    fn set_big_uint64<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, biguint64, value).
        set_view_value::<u64>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [7.2 DataView.prototype.setFloat16 ( byteOffset, value [ , littleEndian ] )](https://tc39.es/proposal-float16array/#sec-dataview.prototype.setfloat16)
    #[cfg(feature = "proposal-float16array")]
    fn set_float16<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope,
    ) -> JsResult<Value> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, float16, value).
        set_view_value::<f16>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [25.3.4.17 DataView.prototype.setFloat32 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setfloat32)
    fn set_float32<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, float32, value).
        set_view_value::<f32>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [25.3.4.18 DataView.prototype.setFloat64 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setfloat64)
    fn set_float64<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, float64, value).
        set_view_value::<f64>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [25.3.4.19 DataView.prototype.setInt8 ( byteOffset, value )](https://tc39.es/ecma262/#sec-dataview.prototype.setint8)
    fn set_int8<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, true, int8, value).
        set_view_value::<i8>(agent, this_value, byte_offset, true, value, gc)
    }

    /// ### [25.3.4.20 DataView.prototype.setInt16 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setint16)
    fn set_int16<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, int16, value).
        set_view_value::<i16>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [25.3.4.21 DataView.prototype.setInt32 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setint32)
    fn set_int32<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, int32, value).
        set_view_value::<i32>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [25.3.4.22 DataView.prototype.setUint8 ( byteOffset, value )](https://tc39.es/ecma262/#sec-dataview.prototype.setuint8)
    fn set_uint8<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, true, uint8, value).
        set_view_value::<u8>(agent, this_value, byte_offset, true, value, gc)
    }

    /// ### [25.3.4.23 DataView.prototype.setUint16 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setuint16)
    fn set_uint16<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, uint16, value).
        set_view_value::<u16>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    /// ### [25.3.4.24 DataView.prototype.setUint32 ( byteOffset, value \[ , littleEndian \] )](https://tc39.es/ecma262/#sec-dataview.prototype.setuint32)
    fn set_uint32<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let byte_offset = arguments.get(0);
        let value = arguments.get(1);
        // 2. If littleEndian is not present, set littleEndian to false.
        let little_endian = to_boolean(agent, arguments.get(2));
        // 1. Let v be the this value.
        // 2. Return ? SetViewValue(v, byteOffset, littleEndian, uint32, value).
        set_view_value::<u32>(agent, this_value, byte_offset, little_endian, value, gc)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.data_view_prototype();
        let data_view_constructor = intrinsics.data_view();

        let mut property_capacity = 25;
        if cfg!(feature = "proposal-float16array") {
            property_capacity += 2;
        }

        let builder = OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(property_capacity)
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
            });

        #[cfg(feature = "proposal-float16array")]
        let builder = builder
            .with_builtin_function_property::<DataViewPrototypeGetFloat16>()
            .with_builtin_function_property::<DataViewPrototypeSetFloat16>();

        builder.build();
    }
}

#[inline]
pub(crate) fn require_internal_slot_data_view<'a>(
    agent: &mut Agent,
    o: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<DataView<'a>> {
    match o {
        // 1. Perform ? RequireInternalSlot(O, [[DataView]]).
        Value::DataView(array_buffer) => Ok(array_buffer.unbind().bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected this to be DataView",
            gc,
        )),
    }
}
