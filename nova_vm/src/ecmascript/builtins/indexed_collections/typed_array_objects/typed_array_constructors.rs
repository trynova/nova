// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::{
    get_iterator_from_method, iterator_to_list,
};
use crate::ecmascript::abstract_operations::operations_on_objects::get_method;
use crate::ecmascript::abstract_operations::type_conversion::to_index;
use crate::ecmascript::builtins::indexed_collections::typed_array_objects::abstract_operations::{
    allocate_typed_array, initialize_typed_array_from_array_buffer,
    initialize_typed_array_from_array_like, initialize_typed_array_from_list,
    initialize_typed_array_from_typed_array,
};
use crate::ecmascript::builtins::typed_array::TypedArray;
use crate::ecmascript::builtins::ArrayBuffer;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::types::{Function, IntoValue, PropertyKey, U8Clamped, Viewable};
use crate::engine::context::GcScope;
use crate::heap::WellKnownSymbolIndexes;
use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct TypedArrayConstructors;

struct Int8ArrayConstructor;
impl Builtin for Int8ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Int8Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::int8_array_constructor);
}
impl BuiltinIntrinsicConstructor for Int8ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Int8Array;
}
struct Uint8ArrayConstructor;
impl Builtin for Uint8ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint8Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint8_array_constructor);
}
impl BuiltinIntrinsicConstructor for Uint8ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Uint8Array;
}
struct Uint8ClampedArrayConstructor;
impl Builtin for Uint8ClampedArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint8ClampedArray;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint8_clamped_array_constructor);
}
impl BuiltinIntrinsicConstructor for Uint8ClampedArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Uint8ClampedArray;
}
struct Int16ArrayConstructor;
impl Builtin for Int16ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Int16Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::int16_array_constructor);
}
impl BuiltinIntrinsicConstructor for Int16ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Int16Array;
}
struct Uint16ArrayConstructor;
impl Builtin for Uint16ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint16Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint16_array_constructor);
}
impl BuiltinIntrinsicConstructor for Uint16ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Uint16Array;
}
struct Int32ArrayConstructor;
impl Builtin for Int32ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Int32Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::int32_array_constructor);
}
impl BuiltinIntrinsicConstructor for Int32ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Int32Array;
}
struct Uint32ArrayConstructor;
impl Builtin for Uint32ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint32Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint32_array_constructor);
}
impl BuiltinIntrinsicConstructor for Uint32ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Uint32Array;
}
struct BigInt64ArrayConstructor;
impl Builtin for BigInt64ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.BigInt64Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::big_int64_array_constructor);
}
impl BuiltinIntrinsicConstructor for BigInt64ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::BigInt64Array;
}
struct BigUint64ArrayConstructor;
impl Builtin for BigUint64ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.BigUint64Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::big_uint64_array_constructor);
}
impl BuiltinIntrinsicConstructor for BigUint64ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::BigUint64Array;
}
struct Float32ArrayConstructor;
impl Builtin for Float32ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Float32Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::float32_array_constructor);
}
impl BuiltinIntrinsicConstructor for Float32ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Float32Array;
}
struct Float64ArrayConstructor;
impl Builtin for Float64ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Float64Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::float64_array_constructor);
}
impl BuiltinIntrinsicConstructor for Float64ArrayConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Float64Array;
}

impl TypedArrayConstructors {
    fn int8_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<i8>(agent, gc, arguments, new_target)
    }

    fn uint8_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<u8>(agent, gc, arguments, new_target)
    }

    fn uint8_clamped_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<U8Clamped>(agent, gc, arguments, new_target)
    }

    fn int16_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<i16>(agent, gc, arguments, new_target)
    }

    fn uint16_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<u16>(agent, gc, arguments, new_target)
    }

    fn int32_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<i32>(agent, gc, arguments, new_target)
    }

    fn uint32_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<u32>(agent, gc, arguments, new_target)
    }

    fn big_int64_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<i64>(agent, gc, arguments, new_target)
    }

    fn big_uint64_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<u64>(agent, gc, arguments, new_target)
    }

    fn float32_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<f32>(agent, gc, arguments, new_target)
    }

    fn float64_array_constructor(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        typed_array_constructor::<f64>(agent, gc, arguments, new_target)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let typed_array_constructor = intrinsics.typed_array().into_object();

        let int8_array_prototype = intrinsics.int8_array_prototype();
        let uint8_array_prototype = intrinsics.uint8_array_prototype();
        let uint8_clamped_array_prototype = intrinsics.uint8_clamped_array_prototype();
        let int16_array_prototype = intrinsics.int16_array_prototype();
        let uint16_array_prototype = intrinsics.uint16_array_prototype();
        let int32_array_prototype = intrinsics.int32_array_prototype();
        let uint32_array_prototype = intrinsics.uint32_array_prototype();
        let big_int64_array_prototype = intrinsics.big_int64_array_prototype();
        let big_uint64_array_prototype = intrinsics.big_uint64_array_prototype();
        let float32_array_prototype = intrinsics.float32_array_prototype();
        let float64_array_prototype = intrinsics.float64_array_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Int8ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(1.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(int8_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint8ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(1.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(uint8_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint8ClampedArrayConstructor>(
            agent, realm,
        )
        .with_property_capacity(2)
        .with_prototype(typed_array_constructor)
        .with_property(|builder| {
            builder
                .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                .with_value_readonly(1.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_prototype_property(uint8_clamped_array_prototype.into_object())
        .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Int16ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(2.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(int16_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint16ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(2.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(uint16_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Int32ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(4.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(int32_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint32ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(4.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(uint32_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<BigInt64ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(8.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(big_int64_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<BigUint64ArrayConstructor>(
            agent, realm,
        )
        .with_property_capacity(2)
        .with_prototype(typed_array_constructor)
        .with_property(|builder| {
            builder
                .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                .with_value_readonly(8.into())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_prototype_property(big_uint64_array_prototype.into_object())
        .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Float32ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(4.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(float32_array_prototype.into_object())
            .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Float64ArrayConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype(typed_array_constructor)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(8.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_prototype_property(float64_array_prototype.into_object())
            .build();
    }
}

pub(crate) struct TypedArrayPrototypes;
impl TypedArrayPrototypes {
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let typed_array_prototype = intrinsics.typed_array_prototype();

        let int8_array_constructor = intrinsics.int8_array();
        let int8_array_prototype = intrinsics.int8_array_prototype();
        let uint8_array_constructor = intrinsics.uint8_array();
        let uint8_array_prototype = intrinsics.uint8_array_prototype();
        let uint8_clamped_array_constructor = intrinsics.uint8_clamped_array();
        let uint8_clamped_array_prototype = intrinsics.uint8_clamped_array_prototype();
        let int16_array_constructor = intrinsics.int16_array();
        let int16_array_prototype = intrinsics.int16_array_prototype();
        let uint16_array_constructor = intrinsics.uint16_array();
        let uint16_array_prototype = intrinsics.uint16_array_prototype();
        let int32_array_constructor = intrinsics.int32_array();
        let int32_array_prototype = intrinsics.int32_array_prototype();
        let uint32_array_constructor = intrinsics.uint32_array();
        let uint32_array_prototype = intrinsics.uint32_array_prototype();
        let big_int64_array_constructor = intrinsics.big_int64_array();
        let big_int64_array_prototype = intrinsics.big_int64_array_prototype();
        let big_uint64_array_constructor = intrinsics.big_uint64_array();
        let big_uint64_array_prototype = intrinsics.big_uint64_array_prototype();
        let float32_array_constructor = intrinsics.float32_array();
        let float32_array_prototype = intrinsics.float32_array_prototype();
        let float64_array_constructor = intrinsics.float64_array();
        let float64_array_prototype = intrinsics.float64_array_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, int8_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(1.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(int8_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, uint8_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(1.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(uint8_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, uint8_clamped_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(1.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(uint8_clamped_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, int16_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(2.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(int16_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, uint16_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(2.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(uint16_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, int32_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(4.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(int32_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, uint32_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(4.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(uint32_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, big_int64_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(8.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(big_int64_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, big_uint64_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(8.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(big_uint64_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, float32_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(4.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(float32_array_constructor)
            .build();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, float64_array_prototype)
            .with_property_capacity(2)
            .with_prototype(typed_array_prototype)
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.BYTES_PER_ELEMENT.into())
                    .with_value_readonly(8.into())
                    .with_enumerable(false)
                    .with_configurable(false)
                    .build()
            })
            .with_constructor_property(float64_array_constructor)
            .build();
    }
}

fn typed_array_constructor<T: Viewable>(
    agent: &mut Agent,
    mut gc: GcScope<'_, '_>,
    arguments: ArgumentsList,
    new_target: Option<Object>,
) -> JsResult<Value> {
    // 1. If NewTarget is undefined, throw a TypeError exception.
    let Some(new_target) = new_target else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "calling a builtin TypedArray constructor without new is forbidden",
        ));
    };
    let new_target = Function::try_from(new_target).unwrap();

    // 2. Let constructorName be the String value of the Constructor Name value specified in Table 69 for this TypedArray constructor.
    // 3. Let proto be "%TypedArray.prototype%".
    let proto = T::PROTO;

    // 4. Let numberOfArgs be the number of elements in args.
    // 5. If numberOfArgs = 0, then
    if arguments.is_empty() {
        // a. Return ? AllocateTypedArray(constructorName, NewTarget, proto, 0).
        return allocate_typed_array::<T>(agent, gc.reborrow(), new_target, proto, Some(0))
            .map(|typed_array| typed_array.into_value());
    }

    // 6. Else,
    // a. Let firstArgument be args[0].
    let first_argument = arguments.get(0);

    // b. If firstArgument is an Object, then
    if first_argument.is_object() {
        // i. Let O be ? AllocateTypedArray(constructorName, NewTarget, proto).
        let o = allocate_typed_array::<T>(agent, gc.reborrow(), new_target, proto, None)?;

        // ii. If firstArgument has a [[TypedArrayName]] internal slot, then
        if let Ok(first_argument) = TypedArray::try_from(o) {
            // 1. Perform ? InitializeTypedArrayFromTypedArray(O, firstArgument).
            match first_argument {
                TypedArray::Int8Array(_) => {
                    initialize_typed_array_from_typed_array::<T, i8>(agent, o, first_argument)?
                }
                TypedArray::Uint8Array(_) => {
                    initialize_typed_array_from_typed_array::<T, u8>(agent, o, first_argument)?
                }
                TypedArray::Uint8ClampedArray(_) => initialize_typed_array_from_typed_array::<
                    T,
                    U8Clamped,
                >(agent, o, first_argument)?,
                TypedArray::Int16Array(_) => {
                    initialize_typed_array_from_typed_array::<T, i16>(agent, o, first_argument)?
                }
                TypedArray::Uint16Array(_) => {
                    initialize_typed_array_from_typed_array::<T, u16>(agent, o, first_argument)?
                }
                TypedArray::Int32Array(_) => {
                    initialize_typed_array_from_typed_array::<T, i32>(agent, o, first_argument)?
                }
                TypedArray::Uint32Array(_) => {
                    initialize_typed_array_from_typed_array::<T, u32>(agent, o, first_argument)?
                }
                TypedArray::BigInt64Array(_) => {
                    initialize_typed_array_from_typed_array::<T, i64>(agent, o, first_argument)?
                }
                TypedArray::BigUint64Array(_) => {
                    initialize_typed_array_from_typed_array::<T, u64>(agent, o, first_argument)?
                }
                TypedArray::Float32Array(_) => {
                    initialize_typed_array_from_typed_array::<T, f32>(agent, o, first_argument)?
                }
                TypedArray::Float64Array(_) => {
                    initialize_typed_array_from_typed_array::<T, f64>(agent, o, first_argument)?
                }
            }
        } else if let Ok(first_argument) = ArrayBuffer::try_from(first_argument) {
            // iii. Else if firstArgument has an [[ArrayBufferData]] internal slot, then
            // 1. If numberOfArgs > 1, let byteOffset be args[1]; else let byteOffset be undefined.
            let byte_offset = if arguments.len() > 1 {
                Some(arguments.get(1))
            } else {
                None
            };

            // 2. If numberOfArgs > 2, let length be args[2]; else let length be undefined.
            let length = if arguments.len() > 2 {
                Some(arguments.get(2))
            } else {
                None
            };

            // 3. Perform ? InitializeTypedArrayFromArrayBuffer(O, firstArgument, byteOffset, length).
            initialize_typed_array_from_array_buffer::<T>(
                agent,
                gc.reborrow(),
                o,
                first_argument,
                byte_offset,
                length,
            )?;
        }
        // iv. Else,

        // 1. Assert: firstArgument is an Object and firstArgument does not have either a [[TypedArrayName]] or an [[ArrayBufferData]] internal slot.
        // 2. Let usingIterator be ? GetMethod(firstArgument, %Symbol.iterator%).
        let using_iterator = get_method(
            agent,
            gc.reborrow(),
            first_argument,
            PropertyKey::Symbol(WellKnownSymbolIndexes::Iterator.into()),
        )?;

        // 3. If usingIterator is not undefined, then
        if let Some(using_iterator) = using_iterator {
            // a. Let values be ? IteratorToList(? GetIteratorFromMethod(firstArgument, usingIterator)).
            let iterator_record =
                &get_iterator_from_method(agent, gc.reborrow(), first_argument, using_iterator)?;
            let values = iterator_to_list(agent, gc.reborrow(), iterator_record)?;
            // b. Perform ? InitializeTypedArrayFromList(O, values).
            initialize_typed_array_from_list::<T>(agent, gc.reborrow(), o, values)?;
        } else {
            // 4. Else,
            // a. NOTE: firstArgument is not an iterable object, so assume it is already an array-like object.
            let first_argument = Object::try_from(first_argument).unwrap();
            // b. Perform ? InitializeTypedArrayFromArrayLike(O, firstArgument).
            initialize_typed_array_from_array_like::<T>(agent, gc.reborrow(), o, first_argument)?;
        }

        // v. Return O.
        return Ok(o.into_value());
    }

    // c. Else,
    // i. Assert: firstArgument is not an Object.
    assert!(!first_argument.is_object());

    // ii. Let elementLength be ? ToIndex(firstArgument).
    let element_length = to_index(agent, gc.reborrow(), first_argument)?;

    // iii. Return ? AllocateTypedArray(constructorName, NewTarget, proto, elementLength).
    allocate_typed_array::<T>(agent, gc, new_target, proto, Some(element_length as usize))
        .map(|typed_array| typed_array.into_value())
}
