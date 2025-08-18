// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{construct, initialize_instance_elements},
            testing_and_comparison::is_constructor,
        },
        builtins::{
            ArgumentsList, BuiltinConstructorFunction, ordinary::ordinary_create_from_constructor,
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics,
            agent::{ExceptionType, unwrap_try},
        },
        types::{Function, InternalMethods, Object},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
};

pub(crate) fn base_class_default_constructor<'a>(
    agent: &mut Agent,
    new_target: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    let new_target = new_target.bind(gc.nogc());
    // ii. If NewTarget is undefined, throw a TypeError exception.
    // Note: We've already checked this at an earlier level.

    // iii. Let F be the active function object.
    let f = BuiltinConstructorFunction::try_from(agent.active_function_object(gc.nogc()))
        .unwrap()
        .scope(agent, gc.nogc());

    // iv. If F.[[ConstructorKind]] is derived, then
    // v. Else,
    // 1. NOTE: This branch behaves similarly to constructor() {}.
    // 2. Let result be ? OrdinaryCreateFromConstructor(NewTarget, "%Object.prototype%").
    let result = ordinary_create_from_constructor(
        agent,
        Function::try_from(new_target.unbind()).unwrap(),
        ProtoIntrinsics::Object,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    let scoped_result = result.scope(agent, gc.nogc());
    // vi. Perform ? InitializeInstanceElements(result, F).
    initialize_instance_elements(agent, result.unbind(), f.get(agent), gc.reborrow()).unbind()?;

    // vii. Return result.
    Ok(scoped_result.get(agent).bind(gc.into_nogc()))
}

pub(crate) fn derived_class_default_constructor<'a>(
    agent: &mut Agent,
    args: ArgumentsList,
    new_target: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    let new_target = new_target.bind(gc.nogc());
    // i. Let args be the List of arguments that was passed to this function by [[Call]] or [[Construct]].
    // ii. If NewTarget is undefined, throw a TypeError exception.
    // Note: We've already checked this at an earlier level.

    // iii. Let F be the active function object.
    let f = BuiltinConstructorFunction::try_from(agent.active_function_object(gc.nogc())).unwrap();

    // iv. If F.[[ConstructorKind]] is derived, then
    // 1. NOTE: This branch behaves similarly to constructor(...args) { super(...args); }.
    // The most notable distinction is that while the aforementioned ECMAScript
    // source text observably calls the %Symbol.iterator% method on
    // %Array.prototype%, this function does not.

    // 2. Let func be ! F.[[GetPrototypeOf]]().
    let func = unwrap_try(f.try_get_prototype_of(agent, gc.nogc()));
    // 3. If IsConstructor(func) is false, throw a TypeError exception.
    let Some(func) = func.and_then(|func| is_constructor(agent, func)) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected callable function",
            gc.into_nogc(),
        ));
    };
    let f = f.scope(agent, gc.nogc());
    // 4. Let result be ? Construct(func, args, NewTarget).
    let result = construct(
        agent,
        func.unbind(),
        Some(args),
        Some(Function::try_from(new_target.unbind()).unwrap()),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    let scoped_result = result.scope(agent, gc.nogc());
    // vi. Perform ? InitializeInstanceElements(result, F).
    initialize_instance_elements(agent, result.unbind(), f.get(agent), gc.reborrow()).unbind()?;

    // vii. Return result.
    Ok(scoped_result.get(agent).bind(gc.into_nogc()))
}
