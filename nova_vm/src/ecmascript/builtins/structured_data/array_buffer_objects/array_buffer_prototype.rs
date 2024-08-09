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

pub(crate) struct ArrayBufferPrototype;

struct ArrayBufferPrototypeGetByteLength;
impl Builtin for ArrayBufferPrototypeGetByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteLength;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.byteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_byte_length);
}
impl BuiltinGetter for ArrayBufferPrototypeGetByteLength {}
struct ArrayBufferPrototypeGetDetached;
impl Builtin for ArrayBufferPrototypeGetDetached {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_detached;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.detached.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_detached);
}
impl BuiltinGetter for ArrayBufferPrototypeGetDetached {}
struct ArrayBufferPrototypeGetMaxByteLength;
impl Builtin for ArrayBufferPrototypeGetMaxByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_maxByteLength;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.maxByteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_max_byte_length);
}
impl BuiltinGetter for ArrayBufferPrototypeGetMaxByteLength {}
struct ArrayBufferPrototypeGetResizable;
impl Builtin for ArrayBufferPrototypeGetResizable {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_resizable;
    const KEY: Option<PropertyKey<'static>> = Some(BUILTIN_STRING_MEMORY.resizable.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::get_resizable);
}
impl BuiltinGetter for ArrayBufferPrototypeGetResizable {}
struct ArrayBufferPrototypeResize;
impl Builtin for ArrayBufferPrototypeResize {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.resize;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::resize);
}
struct ArrayBufferPrototypeSlice;
impl Builtin for ArrayBufferPrototypeSlice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::slice);
}
struct ArrayBufferPrototypeTransfer;
impl Builtin for ArrayBufferPrototypeTransfer {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.transfer;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::transfer);
}
struct ArrayBufferPrototypeTransferToFixedLength;
impl Builtin for ArrayBufferPrototypeTransferToFixedLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.transferToFixedLength;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferPrototype::transfer_to_fixed_length);
}

impl ArrayBufferPrototype {
    fn get_byte_length<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_detached<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_max_byte_length<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn get_resizable<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn resize<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn slice<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn transfer<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _: ArgumentsList) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn transfer_to_fixed_length<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
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
