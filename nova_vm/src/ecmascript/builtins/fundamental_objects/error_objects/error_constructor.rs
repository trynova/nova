// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "proposal-is-error")]
use crate::engine::context::NoGcScope;
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{get, has_property},
            type_conversion::to_string,
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor, error::Error,
            ordinary::ordinary_create_from_constructor,
        },
        execution::{Agent, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, Function, Object, PropertyKey, String, Value},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::{ArenaAccessMut, IntrinsicConstructorIndexes},
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
    /// ### [20.5.1.1 Error ( message \[ , options \] )](https://tc39.es/ecma262/#sec-error-message)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let message = arguments.get(0).bind(gc.nogc());
        let mut options = arguments.get(1).bind(gc.nogc());
        let mut new_target = new_target.bind(gc.nogc());

        // 3. If message is not undefined, then
        let message = if let Ok(message) = String::try_from(message) {
            Some(message.scope(agent, gc.nogc()))
        } else if !message.is_undefined() {
            // a. Let msg be ? ToString(message).
            let scoped_options = options.scope(agent, gc.nogc());
            let scoped_new_target = new_target.map(|n| n.scope(agent, gc.nogc()));
            let message = to_string(agent, message.unbind(), gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());
            // SAFETY: Never shared.
            unsafe {
                new_target = scoped_new_target.map(|n| n.take(agent)).bind(gc.nogc());
                options = scoped_options.take(agent).bind(gc.nogc());
            }
            Some(message)
        } else {
            None
        };
        // 4. Perform ? InstallErrorCause(O, options).
        let cause = if !options.is_object() {
            None
        } else {
            let scoped_new_target = new_target.map(|n| n.scope(agent, gc.nogc()));
            let cause = get_error_cause(agent, options.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: Never shared.
            new_target = unsafe { scoped_new_target.map(|n| n.take(agent)).bind(gc.nogc()) };
            cause.map(|c| c.scope(agent, gc.nogc()))
        };

        // 1. If NewTarget is undefined, let newTarget be the active function object; else let newTarget be NewTarget.
        let new_target = new_target.map_or_else(
            || agent.running_execution_context().function.unwrap(),
            |new_target| Function::try_from(new_target).unwrap(),
        );
        // 2. Let O be ? OrdinaryCreateFromConstructor(newTarget, "%Error.prototype%", « [[ErrorData]] »).
        let o = ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::Error,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.into_nogc());
        let o = Error::try_from(o).unwrap();
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let message = message.map(|message| message.get(agent));
        let cause = cause.map(|c| c.get(agent));
        let heap_data = o.get_mut(agent);
        heap_data.kind = ExceptionType::Error;
        heap_data.message = message;
        heap_data.cause = cause;
        // 5. Return O.
        Ok(o.into())
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
