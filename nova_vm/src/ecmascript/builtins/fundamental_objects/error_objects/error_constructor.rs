// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_objects::get;
use crate::ecmascript::abstract_operations::operations_on_objects::has_property;
use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::builtins::error::Error;
use crate::ecmascript::builtins::ordinary::ordinary_create_from_constructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::PropertyKey;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::engine::context::Bindable;
use crate::engine::context::GcScope;
use crate::engine::rootable::Scopable;
use crate::heap::IntrinsicConstructorIndexes;

pub(crate) struct ErrorConstructor;

impl Builtin for ErrorConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Error;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for ErrorConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Error;
}

impl ErrorConstructor {
    /// ### [20.5.1.1 Error ( message \[ , options \] )](https://tc39.es/ecma262/#sec-error-message)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
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
            let message = to_string(agent, message.unbind(), gc.reborrow())?
                .unbind()
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
            let cause = get_error_cause(agent, options.unbind(), gc.reborrow())?
                .unbind()
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
        )?
        .unbind()
        .bind(gc.into_nogc());
        let o = Error::try_from(o).unwrap();
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let message = message.map(|message| message.get(agent));
        let cause = cause.map(|c| c.get(agent));
        let heap_data = &mut agent[o];
        heap_data.kind = ExceptionType::Error;
        heap_data.message = message;
        heap_data.cause = cause;
        // 5. Return O.
        Ok(o.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let error_prototype = intrinsics.error_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ErrorConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(error_prototype.into_object())
            .build();
    }
}

pub(super) fn get_error_cause<'gc>(
    agent: &mut Agent,
    options: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Option<Value<'gc>>> {
    let Ok(options) = Object::try_from(options) else {
        return Ok(None);
    };
    let key = PropertyKey::from(BUILTIN_STRING_MEMORY.cause);
    if has_property(agent, options, key, gc.reborrow())? {
        Ok(Some(get(agent, options, key, gc)?))
    } else {
        Ok(None)
    }
}
