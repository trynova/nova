use crate::ecmascript::{
    builders::{
        builtin_function_builder::BuiltinFunctionBuilder,
        ordinary_object_builder::OrdinaryObjectBuilder,
    },
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct TypedArrayConstructors;

struct Int8ArrayConstructor;
impl Builtin for Int8ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Int8Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::int8_array_constructor);
}
struct Uint8ArrayConstructor;
impl Builtin for Uint8ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint8Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint8_array_constructor);
}
struct Uint8ClampedArrayConstructor;
impl Builtin for Uint8ClampedArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint8ClampedArray;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint8_clamped_array_constructor);
}
struct Int16ArrayConstructor;
impl Builtin for Int16ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Int16Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::int16_array_constructor);
}
struct Uint16ArrayConstructor;
impl Builtin for Uint16ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint16Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint16_array_constructor);
}
struct Int32ArrayConstructor;
impl Builtin for Int32ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Int32Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::int32_array_constructor);
}
struct Uint32ArrayConstructor;
impl Builtin for Uint32ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Uint32Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::uint32_array_constructor);
}
struct BigInt64ArrayConstructor;
impl Builtin for BigInt64ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.BigInt64Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::big_int64_array_constructor);
}
struct BigUint64ArrayConstructor;
impl Builtin for BigUint64ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.BigUint64Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::big_uint64_array_constructor);
}
struct Float32ArrayConstructor;
impl Builtin for Float32ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Float32Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::float32_array_constructor);
}
struct Float64ArrayConstructor;
impl Builtin for Float64ArrayConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Float64Array;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(TypedArrayConstructors::float64_array_constructor);
}

impl TypedArrayConstructors {
    fn int8_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn uint8_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn uint8_clamped_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn int16_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn uint16_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn int32_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn uint32_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn big_int64_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn big_uint64_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn float32_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn float64_array_constructor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let typed_array_constructor = intrinsics.typed_array().into_object();

        let int8_array_constructor_this = intrinsics.int8_array();
        let int8_array_constructor_base_object = intrinsics.int8_array_base_object();
        let int8_array_prototype = intrinsics.int8_array_prototype();
        let uint8_array_constructor_this = intrinsics.uint8_array();
        let uint8_array_constructor_base_object = intrinsics.uint8_array_base_object();
        let uint8_array_prototype = intrinsics.uint8_array_prototype();
        let uint8_clamped_array_constructor_this = intrinsics.uint8_clamped_array();
        let uint8_clamped_array_constructor_base_object =
            intrinsics.uint8_clamped_array_base_object();
        let uint8_clamped_array_prototype = intrinsics.uint8_clamped_array_prototype();
        let int16_array_constructor_this = intrinsics.int16_array();
        let int16_array_constructor_base_object = intrinsics.int16_array_base_object();
        let int16_array_prototype = intrinsics.int16_array_prototype();
        let uint16_array_constructor_this = intrinsics.uint16_array();
        let uint16_array_constructor_base_object = intrinsics.uint16_array_base_object();
        let uint16_array_prototype = intrinsics.uint16_array_prototype();
        let int32_array_constructor_this = intrinsics.int32_array();
        let int32_array_constructor_base_object = intrinsics.int32_array_base_object();
        let int32_array_prototype = intrinsics.int32_array_prototype();
        let uint32_array_constructor_this = intrinsics.uint32_array();
        let uint32_array_constructor_base_object = intrinsics.uint32_array_base_object();
        let uint32_array_prototype = intrinsics.uint32_array_prototype();
        let big_int64_array_constructor_this = intrinsics.big_int64_array();
        let big_int64_array_constructor_base_object = intrinsics.big_int64_array_base_object();
        let big_int64_array_prototype = intrinsics.big_int64_array_prototype();
        let big_uint64_array_constructor_this = intrinsics.big_uint64_array();
        let big_uint64_array_constructor_base_object = intrinsics.big_uint64_array_base_object();
        let big_uint64_array_prototype = intrinsics.big_uint64_array_prototype();
        let float32_array_constructor_this = intrinsics.float32_array();
        let float32_array_constructor_base_object = intrinsics.float32_array_base_object();
        let float32_array_prototype = intrinsics.float32_array_prototype();
        let float64_array_constructor_this = intrinsics.float64_array();
        let float64_array_constructor_base_object = intrinsics.float64_array_base_object();
        let float64_array_prototype = intrinsics.float64_array_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Int8ArrayConstructor>(
            agent,
            realm,
            int8_array_constructor_this,
            Some(int8_array_constructor_base_object),
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
        .with_prototype_property(int8_array_prototype.into_object())
        .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint8ArrayConstructor>(
            agent,
            realm,
            uint8_array_constructor_this,
            Some(uint8_array_constructor_base_object),
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
        .with_prototype_property(uint8_array_prototype.into_object())
        .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint8ClampedArrayConstructor>(
            agent,
            realm,
            uint8_clamped_array_constructor_this,
            Some(uint8_clamped_array_constructor_base_object),
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

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Int16ArrayConstructor>(
            agent,
            realm,
            int16_array_constructor_this,
            Some(int16_array_constructor_base_object),
        )
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

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint16ArrayConstructor>(
            agent,
            realm,
            uint16_array_constructor_this,
            Some(uint16_array_constructor_base_object),
        )
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

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Int32ArrayConstructor>(
            agent,
            realm,
            int32_array_constructor_this,
            Some(int32_array_constructor_base_object),
        )
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

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Uint32ArrayConstructor>(
            agent,
            realm,
            uint32_array_constructor_this,
            Some(uint32_array_constructor_base_object),
        )
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

        BuiltinFunctionBuilder::new_intrinsic_constructor::<BigInt64ArrayConstructor>(
            agent,
            realm,
            big_int64_array_constructor_this,
            Some(big_int64_array_constructor_base_object),
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
        .with_prototype_property(big_int64_array_prototype.into_object())
        .build();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<BigUint64ArrayConstructor>(
            agent,
            realm,
            big_uint64_array_constructor_this,
            Some(big_uint64_array_constructor_base_object),
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

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Float32ArrayConstructor>(
            agent,
            realm,
            float32_array_constructor_this,
            Some(float32_array_constructor_base_object),
        )
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

        BuiltinFunctionBuilder::new_intrinsic_constructor::<Float64ArrayConstructor>(
            agent,
            realm,
            float64_array_constructor_this,
            Some(float64_array_constructor_base_object),
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
