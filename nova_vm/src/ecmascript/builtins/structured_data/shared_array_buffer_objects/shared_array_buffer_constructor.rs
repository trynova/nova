// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoObject, Object, PropertyKey, String, Value},
    },
    engine::context::{Bindable, GcScope},
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct SharedArrayBufferConstructor;
impl Builtin for SharedArrayBufferConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.SharedArrayBuffer;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for SharedArrayBufferConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::SharedArrayBuffer;
}

struct SharedArrayBufferGetSpecies;
impl Builtin for SharedArrayBufferGetSpecies {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;

    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(SharedArrayBufferConstructor::get_species);
}
impl BuiltinGetter for SharedArrayBufferGetSpecies {}

impl SharedArrayBufferConstructor {
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("SharedArrayBuffer", gc.into_nogc()))
    }

    /// ### [25.2.4.2 get SharedArrayBuffer \[ %Symbol.species% \]](https://tc39.es/ecma262/#sec-sharedarraybuffer-%symbol.species%)
    ///
    /// SharedArrayBuffer\[%Symbol.species%\] is an accessor property whose set
    /// accessor function is undefined.
    ///
    /// > Note: The value of the "name" property of this function is
    /// > "get \[Symbol.species\]".
    fn get_species<'gc>(
        _agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return the this value.
        Ok(this_value.bind(gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let shared_array_buffer_prototype = intrinsics.shared_array_buffer_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<SharedArrayBufferConstructor>(
            agent, realm,
        )
        .with_property_capacity(2)
        .with_prototype_property(shared_array_buffer_prototype.into_object())
        .with_builtin_function_getter_property::<SharedArrayBufferGetSpecies>()
        .build();
    }
}
