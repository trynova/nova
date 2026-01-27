// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, BuiltinFunctionBuilder,
        BuiltinIntrinsicConstructor, Error, ExceptionType, Function, JsResult, Object,
        ProtoIntrinsics, Realm, String, Value, ordinary_create_from_constructor, to_string,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::{ArenaAccessMut, IntrinsicConstructorIndexes},
};

use super::error_constructor::get_error_cause;

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

pub(crate) struct NativeErrorConstructors;
impl NativeErrorConstructors {
    /// ### [20.5.6.1.1 NativeError ( message \[ , options \] )](https://tc39.es/ecma262/#sec-nativeerror)
    #[inline(always)]
    fn constructor<'gc>(
        agent: &mut Agent,
        error_kind: ExceptionType,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let scoped_message = arguments.get(0).scope(agent, nogc);
        let options = arguments.get(1).scope(agent, nogc);
        let new_target = new_target.bind(nogc);

        let intrinsic = match error_kind {
            ExceptionType::Error => ProtoIntrinsics::Error,
            ExceptionType::AggregateError => ProtoIntrinsics::AggregateError,
            ExceptionType::EvalError => ProtoIntrinsics::EvalError,
            ExceptionType::RangeError => ProtoIntrinsics::RangeError,
            ExceptionType::ReferenceError => ProtoIntrinsics::ReferenceError,
            ExceptionType::SyntaxError => ProtoIntrinsics::SyntaxError,
            ExceptionType::TypeError => ProtoIntrinsics::TypeError,
            ExceptionType::UriError => ProtoIntrinsics::URIError,
        };

        // 1. If NewTarget is undefined, let newTarget be the active function
        //    object; else let newTarget be NewTarget.
        let new_target = new_target
            .unwrap_or_else(|| agent.running_execution_context().function.unwrap().into());
        // 2. Let O be ? OrdinaryCreateFromConstructor(newTarget, "%NativeError.prototype%", « [[ErrorData]] »).
        let o = ordinary_create_from_constructor(
            agent,
            Function::try_from(new_target.unbind()).unwrap(),
            intrinsic,
            gc.reborrow(),
        )
        .unbind()?
        .scope(agent, gc.nogc());
        let message = scoped_message.get(agent).bind(gc.nogc());
        // 3. If message is not undefined, then
        let msg = if !message.is_undefined() {
            // a. Let msg be ? ToString(message).
            let msg = to_string(agent, message.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
            // Safety: scoped_message is never shared.
            Some(unsafe { scoped_message.replace_self(agent, msg.unbind()) })
        } else {
            None
        };
        // 4. Perform ? InstallErrorCause(O, options).
        // 5. Return O.
        let cause = get_error_cause(agent, options.get(agent), gc.reborrow()).unbind()?;
        let gc = gc.into_nogc();
        let cause = cause.bind(gc);
        let o = Error::try_from(o.get(agent).bind(gc)).unwrap();
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let msg = msg.map(|msg| msg.get(agent).bind(gc));
        let heap_data = o.get_mut(agent);
        heap_data.kind = error_kind;
        heap_data.message = msg.unbind();
        heap_data.cause = cause.unbind();
        Ok(o.into())
    }

    fn eval_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::constructor(agent, ExceptionType::EvalError, arguments, new_target, gc)
    }

    fn range_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::constructor(agent, ExceptionType::RangeError, arguments, new_target, gc)
    }

    fn reference_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::constructor(
            agent,
            ExceptionType::ReferenceError,
            arguments,
            new_target,
            gc,
        )
    }

    fn syntax_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::constructor(agent, ExceptionType::SyntaxError, arguments, new_target, gc)
    }

    fn type_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::constructor(agent, ExceptionType::TypeError, arguments, new_target, gc)
    }

    fn uri_error_constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::constructor(agent, ExceptionType::UriError, arguments, new_target, gc)
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
