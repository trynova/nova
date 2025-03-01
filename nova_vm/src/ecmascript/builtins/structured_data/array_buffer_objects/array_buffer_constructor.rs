// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::get,
            type_conversion::{to_index, validate_index},
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
            array_buffer::allocate_array_buffer,
        },
        execution::{Agent, JsResult, RealmIdentifier, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, PropertyKey, String,
            Value,
        },
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct ArrayBufferConstructor;
impl Builtin for ArrayBufferConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.ArrayBuffer;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for ArrayBufferConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::ArrayBuffer;
}

struct ArrayBufferIsView;
impl Builtin for ArrayBufferIsView {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isView;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferConstructor::is_view);
}

struct ArrayBufferGetSpecies;
impl Builtin for ArrayBufferGetSpecies {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;

    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferConstructor::species);
}
impl BuiltinGetter for ArrayBufferGetSpecies {}

impl ArrayBufferConstructor {
    // ### [25.1.4.1 ArrayBuffer ( length \[ , options \] )](https://tc39.es/ecma262/#sec-arraybuffer-constructor)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Constructor ArrayBuffer requires 'new'",
                nogc,
            ));
        };
        let new_target = new_target.bind(nogc);
        let length = arguments.get(0).bind(nogc);
        let options = if arguments.len() > 1 {
            Some(arguments.get(1).bind(nogc))
        } else {
            None
        };
        let (byte_length, new_target, requested_max_byte_length) =
            if let (Value::Integer(integer), true) = (length, options.is_none()) {
                (
                    validate_index(agent, integer.into_i64(), nogc)?,
                    new_target,
                    None,
                )
            } else {
                let options = options.map(|o| o.scope(agent, nogc));
                let new_target = new_target.scope(agent, nogc);
                // 2. Let byteLength be ? ToIndex(length).
                let byte_length = to_index(agent, length.unbind(), gc.reborrow())? as u64;
                // 3. Let requestedMaxByteLength be ? GetArrayBufferMaxByteLengthOption(options).
                let requested_max_byte_length = if let Some(options) = options {
                    get_array_buffer_max_byte_length_option(
                        agent,
                        options.get(agent),
                        gc.reborrow(),
                    )?
                } else {
                    None
                };
                (
                    byte_length,
                    new_target.get(agent).bind(gc.nogc()),
                    requested_max_byte_length,
                )
            };
        // 4. Return ? AllocateArrayBuffer(NewTarget, byteLength, requestedMaxByteLength).
        allocate_array_buffer(
            agent,
            Function::try_from(new_target).unwrap().unbind(),
            byte_length,
            requested_max_byte_length,
            gc.into_nogc(),
        )
        .map(|ab| ab.into_value())
    }

    /// ### [25.1.5.1 ArrayBuffer.isView ( arg )](https://tc39.es/ecma262/#sec-arraybuffer.isview)
    fn is_view<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. If arg is not an Object, return false.
        // 2. If arg has a [[ViewedArrayBuffer]] internal slot, return true.
        // 3. Return false.
        Ok(matches!(
            arguments.get(0),
            Value::DataView(_)
                | Value::Uint8Array(_)
                | Value::Uint8ClampedArray(_)
                | Value::Int8Array(_)
                | Value::Uint16Array(_)
                | Value::Int16Array(_)
                | Value::Uint32Array(_)
                | Value::Int32Array(_)
                | Value::BigUint64Array(_)
                | Value::BigInt64Array(_)
                | Value::Float32Array(_)
                | Value::Float64Array(_)
        )
        .into())
    }

    /// ### [25.1.5.3 get ArrayBuffer \[ %Symbol.species% \]](https://tc39.es/ecma262/#sec-get-arraybuffer-%symbol.species%)
    ///
    /// ArrayBuffer\[%Symbol.species%] is an accessor property whose set
    /// accessor function is undefined.
    ///
    /// > ### Note
    /// > `ArrayBuffer.prototype.slice ( start, end )` normally uses its
    /// > **this** value's constructor to create a derived object. However, a
    /// > subclass constructor may over-ride that default behaviour for the
    /// > `ArrayBuffer.prototype.slice ( start, end )` method by redefining its
    /// > `%Symbol.species%` property.
    fn species<'gc>(
        _agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Return the this value.
        // The value of the "name" property of this function is "get [Symbol.species]".
        Ok(this_value.bind(gc.into_nogc()))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let array_buffer_prototype = intrinsics.array_buffer_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ArrayBufferConstructor>(agent, realm)
            .with_property_capacity(3)
            .with_builtin_function_property::<ArrayBufferIsView>()
            .with_prototype_property(array_buffer_prototype.into_object())
            .with_builtin_function_getter_property::<ArrayBufferGetSpecies>()
            .build();
    }
}

/// ### [25.1.3.7 GetArrayBufferMaxByteLengthOption ( options )](https://tc39.es/ecma262/#sec-getarraybuffermaxbytelengthoption)
///
/// The abstract operation GetArrayBufferMaxByteLengthOption takes argument
/// options (an ECMAScript language value) and returns either a normal
/// completion containing either a non-negative integer or empty, or a throw
/// completion.
fn get_array_buffer_max_byte_length_option(
    agent: &mut Agent,
    options: Value,
    mut gc: GcScope,
) -> JsResult<Option<u64>> {
    // 1. If options is not an Object, return empty.
    let Ok(options) = Object::try_from(options) else {
        return Ok(None);
    };
    // 2. Let maxByteLength be ? Get(options, "maxByteLength").
    let max_byte_length = get(
        agent,
        options,
        BUILTIN_STRING_MEMORY.maxByteLength.into(),
        gc.reborrow(),
    )?;
    // 3. If maxByteLength is undefined, return empty.
    if max_byte_length.is_undefined() {
        Ok(None)
    } else {
        // 4. Return ? ToIndex(maxByteLength).
        Ok(Some(to_index(agent, max_byte_length.unbind(), gc)? as u64))
    }
}
