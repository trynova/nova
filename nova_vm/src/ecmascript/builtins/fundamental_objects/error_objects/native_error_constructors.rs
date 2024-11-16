// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_string,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            error::Error, ordinary::ordinary_create_from_constructor, ArgumentsList, Behaviour,
            Builtin, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{Function, IntoObject, IntoValue, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

use super::error_constructor::get_error_cause;

struct EvalErrorConstructor;
impl Builtin for EvalErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.EvalError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::eval_behaviour);
}
impl BuiltinIntrinsicConstructor for EvalErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::EvalError;
}
struct RangeErrorConstructor;
impl Builtin for RangeErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.RangeError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::range_behaviour);
}
impl BuiltinIntrinsicConstructor for RangeErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::RangeError;
}
struct ReferenceErrorConstructor;
impl Builtin for ReferenceErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.ReferenceError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::reference_behaviour);
}
impl BuiltinIntrinsicConstructor for ReferenceErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::ReferenceError;
}
struct SyntaxErrorConstructor;
impl Builtin for SyntaxErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.SyntaxError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::syntax_behaviour);
}
impl BuiltinIntrinsicConstructor for SyntaxErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::SyntaxError;
}
struct TypeErrorConstructor;
impl Builtin for TypeErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.TypeError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::type_behaviour);
}
impl BuiltinIntrinsicConstructor for TypeErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TypeError;
}
struct URIErrorConstructor;
impl Builtin for URIErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.URIError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::uri_behaviour);
}
impl BuiltinIntrinsicConstructor for URIErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::URIError;
}

pub(crate) struct NativeErrorConstructors;
impl NativeErrorConstructors {
    #[inline(always)]
    fn behaviour(
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
        error_kind: ExceptionType,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let message = arguments.get(0);
        let options = arguments.get(1);

        let intrinsic = match error_kind {
            ExceptionType::Error => ProtoIntrinsics::Error,
            ExceptionType::AggregateError => ProtoIntrinsics::AggregateError,
            ExceptionType::EvalError => ProtoIntrinsics::EvalError,
            ExceptionType::RangeError => ProtoIntrinsics::RangeError,
            ExceptionType::ReferenceError => ProtoIntrinsics::ReferenceError,
            ExceptionType::SyntaxError => ProtoIntrinsics::SyntaxError,
            ExceptionType::TypeError => ProtoIntrinsics::TypeError,
            ExceptionType::UriError => ProtoIntrinsics::UriError,
        };

        let new_target = new_target.unwrap_or_else(|| {
            agent
                .running_execution_context()
                .function
                .unwrap()
                .into_object()
        });
        let o = ordinary_create_from_constructor(
            agent,
            gc.reborrow(),
            Function::try_from(new_target).unwrap(),
            intrinsic,
        )?;
        let msg = if !message.is_undefined() {
            Some(to_string(agent, gc.reborrow(), message)?.scope(agent, *gc))
        } else {
            None
        };
        let cause = get_error_cause(agent, gc.reborrow(), options)?;
        let o = Error::try_from(o).unwrap();
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let msg = msg.map(|msg| msg.get_unbound(agent));
        let heap_data = &mut agent[o];
        heap_data.kind = error_kind;
        heap_data.message = msg;
        heap_data.cause = cause;
        Ok(o.into_value())
    }

    fn eval_behaviour(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, gc, ExceptionType::EvalError, arguments, new_target)
    }

    fn range_behaviour(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, gc, ExceptionType::RangeError, arguments, new_target)
    }

    fn reference_behaviour(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(
            agent,
            gc,
            ExceptionType::ReferenceError,
            arguments,
            new_target,
        )
    }

    fn syntax_behaviour(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, gc, ExceptionType::SyntaxError, arguments, new_target)
    }

    fn type_behaviour(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, gc, ExceptionType::TypeError, arguments, new_target)
    }

    fn uri_behaviour(
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, gc, ExceptionType::UriError, arguments, new_target)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let error_constructor = intrinsics.error().into_object();
        let eval_error_prototype = intrinsics.eval_error_prototype();
        let range_error_prototype = intrinsics.range_error_prototype();
        let reference_error_prototype = intrinsics.reference_error_prototype();
        let syntax_error_prototype = intrinsics.syntax_error_prototype();
        let type_error_prototype = intrinsics.type_error_prototype();
        let uri_error_prototype = intrinsics.uri_error_prototype();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<EvalErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(eval_error_prototype.into_object())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<RangeErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(range_error_prototype.into_object())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<ReferenceErrorConstructor>(
            agent, realm,
        )
        .with_property_capacity(1)
        .with_prototype(error_constructor)
        .with_prototype_property(reference_error_prototype.into_object())
        .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<SyntaxErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(syntax_error_prototype.into_object())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<TypeErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(type_error_prototype.into_object())
            .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<URIErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype(error_constructor)
            .with_prototype_property(uri_error_prototype.into_object())
            .build();
    }
}
