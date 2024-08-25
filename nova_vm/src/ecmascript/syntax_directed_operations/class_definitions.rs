use crate::ecmascript::{abstract_operations::{operations_on_objects::construct, testing_and_comparison::is_constructor}, builtins::{ordinary::ordinary_create_from_constructor, ArgumentsList}, execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics}, types::{Function, InternalMethods, Object, Value}};

pub(crate) fn base_class_default_constructor(
        agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
    // ii. If NewTarget is undefined, throw a TypeError exception.
    let Some(new_target) = new_target else {
        return Err(agent.throw_exception_with_static_message(ExceptionType::TypeError, "class constructors must be invoked with 'new'"));
    };
    // iii. Let F be the active function object.
    let _ = agent.running_execution_context().function.unwrap();

    // TODO: Figure out if it's actually okay to split this function into two,
    // or is F's [[ConstructorKind]] only decidable at runtime?

    // iv. If F.[[ConstructorKind]] is derived, then
    // 1. NOTE: This branch behaves similarly to constructor() {}.
    // 2. Let result be ? OrdinaryCreateFromConstructor(NewTarget, "%Object.prototype%").
    let result = ordinary_create_from_constructor(agent, Function::try_from(new_target).unwrap(), ProtoIntrinsics::Object)?;
    // vi. Perform ? InitializeInstanceElements(result, F).
    // vii. Return result.
    Ok(result.into_value())
}

pub(crate) fn derived_class_default_constructor(
        agent: &mut Agent,
        _this_value: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
    // i. Let args be the List of arguments that was passed to this function by [[Call]] or [[Construct]].
    // ii. If NewTarget is undefined, throw a TypeError exception.
    let Some(new_target) = new_target else {
        return Err(agent.throw_exception_with_static_message(ExceptionType::TypeError, "class constructors must be invoked with 'new'"));
    };
    // iii. Let F be the active function object.
    let f = agent.running_execution_context().function.unwrap();

    // TODO: Figure out if it's actually okay to split this function into two,
    // or is F's [[ConstructorKind]] only decidable at runtime?

    // iv. If F.[[ConstructorKind]] is derived, then
    // 1. NOTE: This branch behaves similarly to constructor(...args) { super(...args); }.
    // The most notable distinction is that while the aforementioned ECMAScript
    // source text observably calls the %Symbol.iterator% method on
    // %Array.prototype%, this function does not.

    // 2. Let func be ! F.[[GetPrototypeOf]]().
    let func = f.internal_get_prototype_of(agent).unwrap();
    // 3. If IsConstructor(func) is false, throw a TypeError exception.
    let Some(func) = func.and_then(|func| is_constructor(agent, func)) else {
        return Err(agent.throw_exception_with_static_message(ExceptionType::TypeError, "Expected callable function"));
    };
    // 4. Let result be ? Construct(func, args, NewTarget).
    let result = construct(agent, func, Some(args), Some(Function::try_from(new_target).unwrap()))?;
    // vi. Perform ? InitializeInstanceElements(result, F).
    // vii. Return result.
    Ok(result.into_value())
}