// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "proposal-is-error")]
use crate::engine::NoGcScope;
use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, Error, ErrorHeapData, ExceptionType, Function,
        InternalMethods, JsResult, Object, PropertyDescriptor, PropertyKey, ProtoIntrinsics, Realm,
        String, Value, builders::BuiltinFunctionBuilder, get, has_property,
        ordinary_populate_from_constructor, to_string, unwrap_try,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::{CreateHeapData, IntrinsicConstructorIndexes},
};

pub(crate) struct ErrorConstructor;

impl Builtin for ErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Error;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for ErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Error;
}
#[cfg(feature = "proposal-is-error")]
struct ErrorIsError;
#[cfg(feature = "proposal-is-error")]
impl Builtin for ErrorIsError {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ErrorConstructor::is_error);
}

impl ErrorConstructor {
    /// ### [20.5.6.1.1 NativeError ( message \[ , options \] )](https://tc39.es/ecma262/#sec-nativeerror)
    pub(crate) fn base_constructor<'gc>(
        agent: &mut Agent,
        error_kind: ExceptionType,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Error<'gc>> {
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
        let new_target = new_target.map_or_else(
            || agent.running_execution_context().function.unwrap(),
            |new_target| Function::try_from(new_target).unwrap(),
        );
        // 2. Let O be ? OrdinaryCreateFromConstructor(newTarget, "%NativeError.prototype%", « [[ErrorData]] »).
        let o = agent
            .heap
            // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
            .create(ErrorHeapData::new(error_kind, None, None))
            .bind(gc.nogc());
        let o = ordinary_populate_from_constructor(
            agent,
            o.unbind().into(),
            new_target.unbind(),
            intrinsic,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        let mut o = Error::try_from(o).unwrap();

        // SAFETY: not shared.
        let message = unsafe { scoped_message.take(agent) }.bind(gc.nogc());

        // 3. If message is not undefined, then
        let msg = if let Ok(msg) = String::try_from(message) {
            Some(msg)
        } else if !message.is_undefined() {
            let scoped_o = o.scope(agent, gc.nogc());
            // a. Let msg be ? ToString(message).
            let ms = to_string(agent, message.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: not shared.
            o = unsafe { scoped_o.take(agent) }.bind(gc.nogc());
            Some(ms)
        } else {
            None
        };

        // 3. If message is not undefined, then
        if let Some(msg) = msg {
            // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
            unwrap_try(o.try_define_own_property(
                agent,
                BUILTIN_STRING_MEMORY.message.into(),
                PropertyDescriptor::non_enumerable_data_descriptor(msg),
                None,
                gc.nogc(),
            ));
        }

        // SAFETY: not shared.
        let options = unsafe { options.take(agent) }.bind(gc.nogc());

        // 4. Perform ? InstallErrorCause(O, options).
        let cause = if !options.is_object() {
            None
        } else {
            let scoped_o = o.scope(agent, gc.nogc());
            let cause = get_error_cause(agent, options.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: not shared.
            o = unsafe { scoped_o.take(agent) }.bind(gc.nogc());
            cause
        };
        // 1. If options is an Object and ? HasProperty(options, "cause") is
        //    true, then
        if let Some(cause) = cause {
            // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "cause", cause).
            unwrap_try(o.try_define_own_property(
                agent,
                BUILTIN_STRING_MEMORY.cause.into(),
                PropertyDescriptor::non_enumerable_data_descriptor(cause),
                None,
                gc.nogc(),
            ));
        }

        // 5. Return O.
        Ok(o.unbind())
    }

    /// ### [20.5.1.1 Error ( message \[ , options \] )](https://tc39.es/ecma262/#sec-error-message)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Self::base_constructor(agent, ExceptionType::Error, arguments, new_target, gc)
            .map(Value::from)
    }

    #[cfg(feature = "proposal-is-error")]
    /// ### [20.5.2.1 Error.isError ( arg )](https://tc39.es/proposal-is-error/#sec-error.iserror)
    fn is_error<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        is_error(_agent, arguments.get(0), gc.into_nogc()).map(Value::Boolean)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let error_prototype = intrinsics.error_prototype();

        let mut property_capacity = 1;
        if cfg!(feature = "proposal-is-error") {
            property_capacity += 1;
        }

        let builder =
            BuiltinFunctionBuilder::new_intrinsic_constructor::<ErrorConstructor>(agent, realm)
                .with_property_capacity(property_capacity)
                .with_prototype_property(error_prototype.into());

        #[cfg(feature = "proposal-is-error")]
        let builder = builder.with_builtin_function_property::<ErrorIsError>();

        builder.build();
    }
}

pub(super) fn get_error_cause<'gc>(
    agent: &mut Agent,
    options: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Option<Value<'gc>>> {
    let Ok(options) = Object::try_from(options) else {
        return Ok(None);
    };
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.cause);
    if has_property(agent, options, key, gc.reborrow()).unbind()? {
        Ok(Some(get(agent, options, key, gc)?))
    } else {
        Ok(None)
    }
}

#[cfg(feature = "proposal-is-error")]
/// ### [20.5.8.2 IsError ( argument )]https://tc39.es/proposal-is-error/#sec-iserror
/// The abstract operation IsError takes argument argument (an Ecmascript
/// language value) and returns a Boolean. It returns a boolean indicating
/// whether the argument is a built-in Error instance or not.
pub(super) fn is_error<'a, 'gc>(
    _agent: &mut Agent,
    argument: impl Into<Value<'a>>,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    let argument = argument.into().bind(gc);
    match argument {
        // 1. If argument is not an Object, return false.
        // 2. If argument has an [[ErrorData]] internal slot, return true.
        Value::Error(_) => Ok(true),
        // 3. Return false.
        _ => Ok(false),
    }
}
