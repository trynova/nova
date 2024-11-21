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
            ordinary::ordinary_create_from_constructor, ArgumentsList, BuiltinConstructorFunction,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics},
        types::{Function, InternalMethods, Object},
    },
    engine::context::GcScope,
};

pub(crate) fn base_class_default_constructor(
    agent: &mut Agent,
    mut gc: GcScope<'_, '_>,
    new_target: Object,
) -> JsResult<Object> {
    // ii. If NewTarget is undefined, throw a TypeError exception.
    // Note: We've already checked this at an earlier level.

    // iii. Let F be the active function object.
    let f = BuiltinConstructorFunction::try_from(agent.active_function_object()).unwrap();

    // iv. If F.[[ConstructorKind]] is derived, then
    // v. Else,
    // 1. NOTE: This branch behaves similarly to constructor() {}.
    // 2. Let result be ? OrdinaryCreateFromConstructor(NewTarget, "%Object.prototype%").
    let result = ordinary_create_from_constructor(
        agent,
        gc.reborrow(),
        Function::try_from(new_target).unwrap(),
        ProtoIntrinsics::Object,
    )?;
    // vi. Perform ? InitializeInstanceElements(result, F).
    initialize_instance_elements(agent, gc, result, f)?;

    // vii. Return result.
    Ok(result)
}

pub(crate) fn derived_class_default_constructor(
    agent: &mut Agent,
    mut gc: GcScope<'_, '_>,
    args: ArgumentsList,
    new_target: Object,
) -> JsResult<Object> {
    // i. Let args be the List of arguments that was passed to this function by [[Call]] or [[Construct]].
    // ii. If NewTarget is undefined, throw a TypeError exception.
    // Note: We've already checked this at an earlier level.

    // iii. Let F be the active function object.
    let f = BuiltinConstructorFunction::try_from(agent.active_function_object()).unwrap();

    // iv. If F.[[ConstructorKind]] is derived, then
    // 1. NOTE: This branch behaves similarly to constructor(...args) { super(...args); }.
    // The most notable distinction is that while the aforementioned ECMAScript
    // source text observably calls the %Symbol.iterator% method on
    // %Array.prototype%, this function does not.

    // 2. Let func be ! F.[[GetPrototypeOf]]().
    let func = f.internal_get_prototype_of(agent, gc.reborrow()).unwrap();
    // 3. If IsConstructor(func) is false, throw a TypeError exception.
    let Some(func) = func.and_then(|func| is_constructor(agent, func)) else {
        return Err(agent.throw_exception_with_static_message(
            gc.nogc(),
            ExceptionType::TypeError,
            "Expected callable function",
        ));
    };
    // 4. Let result be ? Construct(func, args, NewTarget).
    let result = construct(
        agent,
        gc.reborrow(),
        func,
        Some(args),
        Some(Function::try_from(new_target).unwrap()),
    )?;
    // vi. Perform ? InitializeInstanceElements(result, F).
    initialize_instance_elements(agent, gc, result, f)?;

    // vii. Return result.
    Ok(result)
}
