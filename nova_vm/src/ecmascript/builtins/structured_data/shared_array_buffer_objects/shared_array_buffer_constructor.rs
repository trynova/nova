// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinFunctionBuilder,
        BuiltinGetter, BuiltinIntrinsicConstructor, ExceptionType, JsResult, Object, PropertyKey,
        Realm, String, Value, get_array_buffer_max_byte_length_option, to_index,
    },
    engine::{
        Bindable, GcScope,
        Scopable,
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

use super::allocate_shared_array_buffer;

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
    /// ### [25.2.3.1 SharedArrayBuffer ( length \[ , options \] )](https://tc39.es/ecma262/#sec-sharedarraybuffer-constructor)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let arguments = arguments.bind(gc.nogc());
        let new_target = new_target.bind(gc.nogc());
        let length = arguments.get(0);
        let options = arguments.get(1).scope(agent, gc.nogc());
        // 1. If NewTarget is undefined,
        let Some(new_target) = new_target else {
            // throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "SharedArrayBuffer Constructor requires 'new'",
                gc.into_nogc(),
            ));
        };
        let new_target = new_target.scope(agent, gc.nogc());
        // 2. Let byteLength be ? ToIndex(length).
        let byte_length = to_index(agent, length.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc()) as u64;
        // 3. Let requestedMaxByteLength be ? GetArrayBufferMaxByteLengthOption(options).
        let requested_max_byte_length = get_array_buffer_max_byte_length_option(
            agent,
            unsafe { options.take(agent) },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 4. Return ? AllocateSharedArrayBuffer(NewTarget, byteLength, requestedMaxByteLength).
        allocate_shared_array_buffer(
            agent,
            // SAFETY: not shared.
            unsafe { new_target.take(agent) },
            byte_length,
            requested_max_byte_length,
            gc,
        )
        .map(|sab| sab.into())
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
        .with_prototype_property(shared_array_buffer_prototype.into())
        .with_builtin_function_getter_property::<SharedArrayBufferGetSpecies>()
        .build();
    }
}
