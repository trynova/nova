// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, ErrorConstructor, ExceptionType, JsResult, Object, Realm,
        String, Value, builders::BuiltinFunctionBuilder,
    },
    engine::GcScope,
    heap::IntrinsicConstructorIndexes,
};

struct EvalErrorConstructor;
impl Builtin for EvalErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.EvalError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::eval_error_constructor);
}
impl BuiltinIntrinsicConstructor for EvalErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::EvalError;
}
struct RangeErrorConstructor;
impl Builtin for RangeErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.RangeError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::range_error_constructor);
}
impl BuiltinIntrinsicConstructor for RangeErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::RangeError;
}
struct ReferenceErrorConstructor;
impl Builtin for ReferenceErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.ReferenceError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::reference_error_constructor);
}
impl BuiltinIntrinsicConstructor for ReferenceErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::ReferenceError;
}
struct SyntaxErrorConstructor;
impl Builtin for SyntaxErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.SyntaxError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::syntax_error_constructor);
}
impl BuiltinIntrinsicConstructor for SyntaxErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::SyntaxError;
}
struct TypeErrorConstructor;
impl Builtin for TypeErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.TypeError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::type_error_constructor);
}
impl BuiltinIntrinsicConstructor for TypeErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TypeError;
}
struct URIErrorConstructor;
impl Builtin for URIErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.URIError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::uri_error_constructor);
}
impl BuiltinIntrinsicConstructor for URIErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::URIError;
}

/// ### [20.5.6.1.1 NativeError ( message \[ , options \] )](https://tc39.es/ecma262/#sec-nativeerror)
pub(crate) struct NativeErrorConstructors;
impl NativeErrorConstructors {
    fn eval_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        ErrorConstructor::base_constructor(
            agent,
            ExceptionType::EvalError,
            arguments,
            new_target,
            gc,
        )
        .map(Value::from)
    }

    fn range_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        ErrorConstructor::base_constructor(
            agent,
            ExceptionType::RangeError,
            arguments,
            new_target,
            gc,
        )
        .map(Value::from)
    }

    fn reference_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        ErrorConstructor::base_constructor(
            agent,
            ExceptionType::ReferenceError,
            arguments,
            new_target,
            gc,
        )
        .map(Value::from)
    }

    fn syntax_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        ErrorConstructor::base_constructor(
            agent,
            ExceptionType::SyntaxError,
            arguments,
            new_target,
            gc,
        )
        .map(Value::from)
    }

    fn type_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        ErrorConstructor::base_constructor(
            agent,
            ExceptionType::TypeError,
            arguments,
            new_target,
            gc,
        )
        .map(Value::from)
    }

    fn uri_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        ErrorConstructor::base_constructor(
            agent,
            ExceptionType::UriError,
            arguments,
            new_target,
            gc,
        )
        .map(Value::from)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let error_constructor = intrinsics.error();
        let eval_error_prototype = intrinsics.eval_error_prototype();
        let range_error_prototype = intrinsics.range_error_prototype();
        let reference_error_prototype = intrinsics.reference_error_prototype();
        let syntax_error_prototype = intrinsics.syntax_error_prototype();
        let type_error_prototype = intrinsics.type_error_prototype();
        let uri_error_prototype = intrinsics.uri_error_prototype();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<EvalErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(eval_error_prototype.into())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<RangeErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(range_error_prototype.into())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<ReferenceErrorConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype(error_constructor)
        .with_prototype_property(reference_error_prototype.into())
        .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<SyntaxErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(syntax_error_prototype.into())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<TypeErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(type_error_prototype.into())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<URIErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(uri_error_prototype.into())
            .build();
    }
}
