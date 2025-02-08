// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct SharedArrayBufferPrototype;

struct SharedArrayBufferPrototypeGetByteLength;
impl Builtin for SharedArrayBufferPrototypeGetByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_byteLength;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.byteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::get_byte_length);
}
impl BuiltinGetter for SharedArrayBufferPrototypeGetByteLength {}
struct SharedArrayBufferPrototypeGrow;
impl Builtin for SharedArrayBufferPrototypeGrow {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.grow;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::grow);
}
struct SharedArrayBufferPrototypeGetGrowable;
impl Builtin for SharedArrayBufferPrototypeGetGrowable {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_growable;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.growable.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::get_growable);
}
impl BuiltinGetter for SharedArrayBufferPrototypeGetGrowable {}
struct SharedArrayBufferPrototypeGetMaxByteLength;
impl Builtin for SharedArrayBufferPrototypeGetMaxByteLength {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get_maxByteLength;
    const KEY: Option<PropertyKey<'static>> =
        Some(BUILTIN_STRING_MEMORY.maxByteLength.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(SharedArrayBufferPrototype::get_max_byte_length);
}
impl BuiltinGetter for SharedArrayBufferPrototypeGetMaxByteLength {}
struct SharedArrayBufferPrototypeSlice;
impl Builtin for SharedArrayBufferPrototypeSlice {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.slice;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferPrototype::slice);
}

impl SharedArrayBufferPrototype {
    fn get_byte_length<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn grow<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_growable<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn get_max_byte_length<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    fn slice<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.shared_array_buffer_prototype();
        let shared_array_buffer_constructor = intrinsics.shared_array_buffer();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(7)
            .with_prototype(object_prototype)
            .with_builtin_function_getter_property::<SharedArrayBufferPrototypeGetByteLength>()
            .with_constructor_property(shared_array_buffer_constructor)
            .with_builtin_function_property::<SharedArrayBufferPrototypeGrow>()
            .with_builtin_function_getter_property::<SharedArrayBufferPrototypeGetGrowable>()
            .with_builtin_function_getter_property::<SharedArrayBufferPrototypeGetMaxByteLength>()
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
