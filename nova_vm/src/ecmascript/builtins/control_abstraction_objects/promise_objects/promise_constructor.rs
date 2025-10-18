// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                IteratorRecord, MaybeInvalidIteratorRecord, get_iterator,
                iterator_close_with_error, iterator_step_value,
            },
            operations_on_objects::{call, call_function, get, throw_not_callable},
            testing_and_comparison::{is_callable, is_constructor},
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
            array_create,
            ordinary::ordinary_create_from_constructor,
            promise::{
                Promise,
                data::{PromiseHeapData, PromiseState},
            },
            promise_objects::{
                promise_abstract_operations::{
                    promise_capability_records::if_abrupt_reject_promise_m,
                    promise_group_record::{PromiseGroupRecord, PromiseGroupType},
                    promise_reaction_records::PromiseReactionHandler,
                },
                promise_prototype::inner_promise_then,
            },
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics, Realm,
            agent::{ExceptionType, JsError},
        },
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyKey, String, Value,
        },
    },
    engine::{
        Scoped,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{CreateHeapData, IntrinsicConstructorIndexes, ObjectEntry, WellKnownSymbolIndexes},
};

use super::promise_abstract_operations::{
    promise_capability_records::PromiseCapability,
    promise_resolving_functions::{PromiseResolvingFunctionHeapData, PromiseResolvingFunctionType},
};

pub(crate) struct PromiseConstructor;
impl Builtin for PromiseConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Promise;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for PromiseConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Promise;
}
struct PromiseAll;
impl Builtin for PromiseAll {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.all;
}
struct PromiseAllSettled;
impl Builtin for PromiseAllSettled {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::all_settled);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.allSettled;
}
struct PromiseAny;
impl Builtin for PromiseAny {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::any);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.any;
}
struct PromiseRace;
impl Builtin for PromiseRace {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::race);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.race;
}
struct PromiseReject;
impl Builtin for PromiseReject {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::reject);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.reject;
}
struct PromiseResolve;
impl Builtin for PromiseResolve {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::resolve);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.resolve;
}
struct PromiseTry;
impl Builtin for PromiseTry {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::r#try);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.r#try;
}
struct PromiseWithResolvers;
impl Builtin for PromiseWithResolvers {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::with_resolvers);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.withResolvers;
}
struct PromiseGetSpecies;
impl Builtin for PromiseGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromiseConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());
}
impl BuiltinGetter for PromiseGetSpecies {}

impl PromiseConstructor {
    /// ### [27.2.3.1 Promise ( executor )](https://tc39.es/ecma262/#sec-promise-executor)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let executor = args.get(0).bind(gc.nogc());

        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Promise Constructor requires 'new'",
                gc.into_nogc(),
            ));
        };
        let new_target = new_target.unbind().bind(gc.nogc());

        if new_target
            != agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_object()
        {
            return Err(throw_promise_subclassing_not_supported(
                agent,
                gc.into_nogc(),
            ));
        }

        // 2. If IsCallable(executor) is false, throw a TypeError exception.
        // TODO: Callable proxies
        let Ok(executor) = Function::try_from(executor) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Not a callable value",
                gc.into_nogc(),
            ));
        };
        let executor = executor.unbind().scope(agent, gc.nogc());

        // 3. Let promise be ? OrdinaryCreateFromConstructor(NewTarget, "%Promise.prototype%", « [[PromiseState]], [[PromiseResult]], [[PromiseFulfillReactions]], [[PromiseRejectReactions]], [[PromiseIsHandled]] »).
        // 4. Set promise.[[PromiseState]] to pending.
        // 5. Set promise.[[PromiseFulfillReactions]] to a new empty List.
        // 6. Set promise.[[PromiseRejectReactions]] to a new empty List.
        // 7. Set promise.[[PromiseIsHandled]] to false.
        let Object::Promise(promise) = ordinary_create_from_constructor(
            agent,
            Function::try_from(new_target.unbind()).unwrap(),
            ProtoIntrinsics::Promise,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc()) else {
            unreachable!()
        };
        let promise = promise.unbind().bind(gc.nogc());
        let scoped_promise = promise.scope(agent, gc.nogc());

        // 8. Let resolvingFunctions be CreateResolvingFunctions(promise).
        let promise_capability = PromiseCapability::from_promise(promise, true);
        let resolve_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Resolve,
            })
            .into_value();
        let reject_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Reject,
            })
            .into_value();

        // 9. Let completion be Completion(Call(executor, undefined, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »)).
        // 10. If completion is an abrupt completion, then
        if let Err(err) = call_function(
            agent,
            executor.get(agent),
            Value::Undefined,
            Some(ArgumentsList::from_mut_slice(&mut [
                resolve_function.unbind(),
                reject_function.unbind(),
            ])),
            gc.reborrow(),
        ) {
            // a. Perform ? Call(resolvingFunctions.[[Reject]], undefined, « completion.[[Value]] »).
            let promise_capability =
                PromiseCapability::from_promise(scoped_promise.get(agent), true);
            promise_capability.reject(agent, err.value().unbind(), gc.nogc());
        }

        // 11. Return promise.
        Ok(scoped_promise.get(agent).into_value())
    }

    /// ### [27.2.4.1 Promise.all ( iterable )](https://tc39.es/ecma262/#sec-promise.all)
    ///
    /// > NOTE: This function requires its **this** value to be a constructor
    /// > function that supports the parameter conventions of the Promise
    /// > constructor.
    fn all<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        promise_group(
            agent,
            this_value,
            arguments,
            PromiseGroupType::PromiseAll,
            gc,
        )
    }

    /// ### [27.2.4.2 Promise.allSettled ( iterable )](https://tc39.es/ecma262/#sec-promise.allsettled)
    ///
    /// > NOTE: This function requires its this value to be a constructor
    /// > function that supports the parameter conventions of the Promise
    /// > constructor.
    fn all_settled<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        promise_group(
            agent,
            this_value,
            arguments,
            PromiseGroupType::PromiseAllSettled,
            gc,
        )
    }

    /// ### [27.2.4.3 Promise.any ( iterable )](https://tc39.es/ecma262/#sec-promise.any)
    ///
    /// > NOTE: This function requires its this value to be a constructor
    /// > function that supports the parameter conventions of the Promise
    /// > constructor.
    fn any<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Promise.any", gc.into_nogc()))
    }

    /// ### [27.2.4.5 Promise.race ( iterable )](https://tc39.es/ecma262/#sec-promise.race)
    ///
    /// > NOTE 1: If the iterable argument yields no values or if none of the
    /// > promises yielded by iterable ever settle, then the pending promise
    /// > returned by this method will never be settled.
    ///
    /// > NOTE 2: This function expects its this value to be a constructor
    /// > function that supports the parameter conventions of the Promise
    /// > constructor. It also expects that its this value provides a resolve
    /// > method.
    fn race<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Promise.race", gc.into_nogc()))
    }

    /// ### [27.2.4.6 Promise.reject ( r )](https://tc39.es/ecma262/#sec-promise.reject)
    ///
    /// > NOTE: This function expects its this value to be a constructor
    /// > function that supports the parameter conventions of the Promise
    /// > constructor.
    fn reject<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let r = arguments.get(0).bind(gc);
        if this_value
            != agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value()
        {
            return Err(throw_promise_subclassing_not_supported(agent, gc));
        }

        // 1. Let C be the this value.
        // 2. Let promiseCapability be ? NewPromiseCapability(C).
        // 3. Perform ? Call(promiseCapability.[[Reject]], undefined, « r »).
        // 4. Return promiseCapability.[[Promise]].
        // NOTE: Since we don't support promise subclassing, this is equivalent
        // to creating an already-rejected promise.
        Ok(Promise::new_rejected(agent, r, gc).into_value())
    }

    /// ### [27.2.4.7 Promise.resolve ( x )](https://tc39.es/ecma262/#sec-promise.resolve)
    ///
    /// > NOTE: This function expects its this value to be a constructor
    /// > function that supports the parameter conventions of the Promise
    /// > constructor.
    fn resolve<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        if this_value
            != agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value()
        {
            return Err(throw_promise_subclassing_not_supported(
                agent,
                gc.into_nogc(),
            ));
        }

        // 3. Return ? PromiseResolve(C, x).
        Ok(Promise::resolve(agent, arguments.get(0), gc).into_value())
    }

    /// ### [27.2.4.8 Promise.try ( callback, ...args )](https://tc39.es/ecma262/#sec-promise.try)
    ///
    /// > NOTE: This function expects its this value to be a constructor
    /// > function that supports the parameter conventions of the Promise
    /// > constructor.
    fn r#try<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let callback_fn = arguments.get(0).bind(gc.nogc());
        let args = arguments.slice_from(1).bind(gc.nogc());
        // 1. Let C be the this value.
        // 2. If C is not an Object, throw a TypeError exception.
        if is_constructor(agent, this_value).is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected the this value to be a constructor.",
                gc.into_nogc(),
            ));
        }
        if this_value
            != agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value()
        {
            return Err(throw_promise_subclassing_not_supported(
                agent,
                gc.into_nogc(),
            ));
        }

        // 3. Let promiseCapability be ? NewPromiseCapability(C).
        // 4. Let status be Completion(Call(callbackfn, undefined, args)).
        let status = call(
            agent,
            callback_fn.unbind(),
            Value::Undefined,
            Some(args.unbind()),
            gc.reborrow(),
        );
        let promise = match status {
            // 5. If status is an abrupt completion, then
            Err(err) => {
                // a. Perform ? Call(promiseCapability.[[Reject]], undefined, « status.[[Value]] »).
                // 7. Return promiseCapability.[[Promise]].
                agent.heap.create(PromiseHeapData {
                    object_index: None,
                    promise_state: PromiseState::Rejected {
                        promise_result: err.value(),
                        is_handled: false,
                    },
                })
            }
            // 6. Else,
            Ok(result) => {
                // a. Perform ? Call(promiseCapability.[[Resolve]], undefined, « status.[[Value]] »).
                Promise::resolve(agent, result.unbind(), gc)
            }
        };
        // 7. Return promiseCapability.[[Promise]].
        Ok(promise.into_value().unbind())
    }

    /// ### [27.2.4.9 Promise.withResolvers ( )](https://tc39.es/ecma262/#sec-promise.withResolvers)
    fn with_resolvers<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        // Step 2 will throw if `this_value` is not a constructor.
        if is_constructor(agent, this_value).is_none() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Expected the this value to be a constructor.",
                gc,
            ));
        }
        if this_value
            != agent
                .current_realm_record()
                .intrinsics()
                .promise()
                .into_value()
        {
            return Err(throw_promise_subclassing_not_supported(agent, gc));
        }

        // 1. Let C be the this value.
        // 2. Let promiseCapability be ? NewPromiseCapability(C).
        let promise_capability = PromiseCapability::new(agent, gc);
        let resolve_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Resolve,
            })
            .into_value();
        let reject_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability: promise_capability.clone(),
                resolve_type: PromiseResolvingFunctionType::Reject,
            })
            .into_value();

        // 3. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        // 4. Perform ! CreateDataPropertyOrThrow(obj, "promise", promiseCapability.[[Promise]]).
        // 5. Perform ! CreateDataPropertyOrThrow(obj, "resolve", promiseCapability.[[Resolve]]).
        // 6. Perform ! CreateDataPropertyOrThrow(obj, "reject", promiseCapability.[[Reject]]).
        let obj = OrdinaryObject::create_object(
            agent,
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into_object(),
            ),
            &[
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.promise.into(),
                    promise_capability.promise().into_value(),
                ),
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.resolve.into(),
                    resolve_function.into_value(),
                ),
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.reject.into(),
                    reject_function.into_value(),
                ),
            ],
        );

        // 7. Return obj.
        Ok(obj.into_value())
    }

    /// ### [27.2.4.10 get Promise \[ %Symbol.species% \]](https://tc39.es/ecma262/#sec-get-promise-%symbol.species%)
    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(this_value.unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let promise_prototype = intrinsics.promise_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<PromiseConstructor>(agent, realm)
            .with_property_capacity(10)
            .with_builtin_function_property::<PromiseAll>()
            .with_builtin_function_property::<PromiseAllSettled>()
            .with_builtin_function_property::<PromiseAny>()
            .with_prototype_property(promise_prototype.into_object())
            .with_builtin_function_property::<PromiseRace>()
            .with_builtin_function_property::<PromiseReject>()
            .with_builtin_function_property::<PromiseResolve>()
            .with_builtin_function_property::<PromiseTry>()
            .with_builtin_function_property::<PromiseWithResolvers>()
            .with_builtin_function_getter_property::<PromiseGetSpecies>()
            .build();
    }
}

/// ### [27.2.4.1.1 GetPromiseResolve ( promiseConstructor )](https://tc39.es/ecma262/#sec-getpromiseresolve)
/// The abstract operation GetPromiseResolve takes argument promiseConstructor
/// (a constructor) and returns either a normal completion containing a function
/// object or a throw completion.
fn get_promise_resolve<'gc>(
    agent: &mut Agent,
    promise_constructor: Function,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Function<'gc>> {
    let promise_constructor = promise_constructor.bind(gc.nogc());

    // 1. Let promiseResolve be ? Get(promiseConstructor, "resolve").
    let promise_resolve = get(
        agent,
        promise_constructor.unbind(),
        BUILTIN_STRING_MEMORY.resolve.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());

    // 2. If IsCallable(promiseResolve) is false, throw a TypeError exception.
    let Some(promise_resolve) = is_callable(promise_resolve, gc.nogc()) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Resolve function is not callable",
            gc.into_nogc(),
        ));
    };

    // 3. Return promiseResolve.
    Ok(promise_resolve.unbind())
}

fn promise_group<'gc>(
    agent: &mut Agent,
    this_value: Value,
    arguments: ArgumentsList,
    promise_group_type: PromiseGroupType,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let this_value = this_value.bind(gc.nogc());
    let arguments = arguments.bind(gc.nogc());
    let iterable = arguments.get(0).scope(agent, gc.nogc());

    // 1. Let C be the this value.
    if this_value
        != agent
            .current_realm_record()
            .intrinsics()
            .promise()
            .into_value()
    {
        return Err(throw_promise_subclassing_not_supported(
            agent,
            gc.into_nogc(),
        ));
    }

    // 2. Let promiseCapability be ? NewPromiseCapability(C).
    let Some(constructor) = is_constructor(agent, this_value) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected the this value to be a constructor.",
            gc.into_nogc(),
        ));
    };
    let constructor = constructor.scope(agent, gc.nogc());
    let promise_capability = PromiseCapability::new(agent, gc.nogc());
    let promise = promise_capability.promise().scope(agent, gc.nogc());

    // 3. Let promiseResolve be Completion(GetPromiseResolve(C)).
    let promise_resolve = get_promise_resolve(agent, constructor.get(agent), gc.reborrow())
        .unbind()
        .bind(gc.nogc());

    // 4. IfAbruptRejectPromise(promiseResolve, promiseCapability).
    let promise_capability = PromiseCapability {
        promise: promise.get(agent).bind(gc.nogc()),
        must_be_unresolved: true,
    };
    let promise_resolve =
        if_abrupt_reject_promise_m!(agent, promise_resolve, promise_capability, gc);
    let promise_resolve = promise_resolve.scope(agent, gc.nogc());

    // 5. Let iteratorRecord be Completion(GetIterator(iterable, sync)).
    let iterator_record = get_iterator(agent, iterable.get(agent), false, gc.reborrow())
        .unbind()
        .bind(gc.nogc());

    // 6. IfAbruptRejectPromise(iteratorRecord, promiseCapability).
    let promise_capability = PromiseCapability {
        promise: promise.get(agent).bind(gc.nogc()),
        must_be_unresolved: true,
    };
    let MaybeInvalidIteratorRecord {
        iterator,
        next_method,
    } = if_abrupt_reject_promise_m!(agent, iterator_record, promise_capability, gc);

    let iterator = iterator.scope(agent, gc.nogc());

    // 7. Let result be Completion(PerformPromiseAll(iteratorRecord, C, promiseCapability, promiseResolve)).
    let mut iterator_done = false;
    let result = perform_promise_group(
        agent,
        iterator.clone(),
        next_method.unbind(),
        constructor,
        promise_capability.unbind(),
        promise_resolve,
        &mut iterator_done,
        promise_group_type,
        gc.reborrow(),
    )
    .unbind()
    .bind(gc.nogc());

    // 8. If result is an abrupt completion, then
    let result = match result {
        Err(mut result) => {
            // a. If iteratorRecord.[[Done]] is false, set result to Completion(IteratorClose(iteratorRecord, result)).
            if !iterator_done {
                result = iterator_close_with_error(
                    agent,
                    iterator.get(agent),
                    result.unbind(),
                    gc.reborrow(),
                )
                .unbind()
                .bind(gc.nogc());
            }

            // b. IfAbruptRejectPromise(result, promiseCapability).
            let promise_capability = PromiseCapability {
                promise: promise.get(agent).bind(gc.nogc()),
                must_be_unresolved: true,
            };
            // a. Perform ? Call(capability.[[Reject]], undefined, « value.[[Value]] »).
            promise_capability.reject(agent, result.value().unbind(), gc.nogc());
            // b. Return capability.[[Promise]].
            promise_capability.promise()
        }
        Ok(result) => result,
    };
    // 9. Return ! result.
    Ok(result.into_value().unbind())
}

/// ### [27.2.4.1.2 PerformPromiseAll ( iteratorRecord, constructor, resultCapability, promiseResolve )](https://tc39.es/ecma262/#sec-performpromiseall)
/// ### [27.2.4.2.1 PerformPromiseAllSettled ( iteratorRecord, constructor, resultCapability, promiseResolve )](https://tc39.es/ecma262/#sec-performpromiseallsettled)
#[allow(clippy::too_many_arguments)]
fn perform_promise_group<'gc>(
    agent: &mut Agent,
    iterator: Scoped<Object>,
    next_method: Option<Function>,
    constructor: Scoped<Function>,
    result_capability: PromiseCapability,
    promise_resolve: Scoped<Function>,
    iterator_done: &mut bool,
    promise_group_type: PromiseGroupType,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Promise<'gc>> {
    let result_capability = result_capability.bind(gc.nogc());

    let Some(next_method) = next_method else {
        return Err(throw_not_callable(agent, gc.into_nogc()));
    };

    let next_method = next_method.scope(agent, gc.nogc());

    // 1. Let values be a new empty List.
    let capacity = match iterator.get(agent) {
        Object::Array(array) => array.len(agent),
        Object::Map(map) => agent[map].size(),
        Object::Set(set) => agent[set].size(),
        _ => 0,
    };

    let result_array = array_create(agent, 0, capacity as usize, None, gc.nogc())
        .unbind()?
        .bind(gc.nogc());
    let result_array = result_array.scope(agent, gc.nogc());

    // 2. Let remainingElementsCount be the Record { [[Value]]: 1 }.
    let promise = result_capability.promise.scope(agent, gc.nogc());
    let promise_group_reference = agent
        .heap
        .create(PromiseGroupRecord {
            promise_group_type: promise_group_type,
            remaining_elements_count: 1,
            result_array: result_array.get(agent),
            promise: promise.get(agent),
        })
        .scope(agent, gc.nogc());

    // 3. Let index be 0.
    let mut index = 0;

    // 4. Repeat,
    loop {
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent),
            next_method: next_method.get(agent),
        }
        .bind(gc.nogc());

        // a. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(agent, iterator_record.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // b. If next is done, then
        let Some(next) = next else {
            *iterator_done = true;
            let promise_group = promise_group_reference.get(agent).bind(gc.nogc());
            let data = promise_group.get_mut(agent);

            // i. Set remainingElementsCount.[[Value]] to remainingElementsCount.[[Value]] - 1.
            data.remaining_elements_count -= 1;

            // ii. If remainingElementsCount.[[Value]] = 0, then
            if data.remaining_elements_count == 0 {
                // 1. Let valuesArray be CreateArrayFromList(values).
                let values_array = result_array.get(agent).bind(gc.nogc());
                // 2. Perform ? Call(resultCapability.[[Resolve]], undefined, « valuesArray »).
                let result_capability = PromiseCapability {
                    promise: promise.get(agent).bind(gc.nogc()),
                    must_be_unresolved: true,
                };
                result_capability.unbind().resolve(
                    agent,
                    values_array.into_value().unbind(),
                    gc.reborrow(),
                );
            }

            // iii. Return resultCapability.[[Promise]].
            return Ok(promise.get(agent));
        };

        // c. Append undefined to values.
        let temp_array = result_array.get(agent).bind(gc.nogc());
        if let Err(err) = temp_array.reserve(agent, 1) {
            return Err(agent.throw_allocation_exception(err, gc.into_nogc()));
        }
        // SAFETY: reserve did not fail.
        unsafe { temp_array.set_len(agent, index + 1) };

        // d. Let nextPromise be ? Call(promiseResolve, constructor, « next »).
        let call_result = call_function(
            agent,
            promise_resolve.get(agent),
            constructor.get(agent).into_value(),
            Some(ArgumentsList::from_mut_value(&mut next.unbind())),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());

        // Note: as we don't yet support Promise subclassing, if we see a
        // non-Promise return we wrap it inside a resolved Promise to get
        // then-chaining.
        let next_promise = match call_result {
            Value::Promise(next_promise) => next_promise,
            _ => Promise::new_resolved(agent, call_result),
        };

        // e. Let steps be the algorithm steps defined in Promise.all Resolve Element Functions.
        // f. Let length be the number of non-optional parameters of the function definition in Promise.all Resolve Element Functions.
        // g. Let onFulfilled be CreateBuiltinFunction(steps, length, "", « [[AlreadyCalled]], [[Index]], [[Values]], [[Capability]], [[RemainingElements]] »).
        // h. Set onFulfilled.[[AlreadyCalled]] to false.
        // i. Set onFulfilled.[[Index]] to index.
        // j. Set onFulfilled.[[Values]] to values.
        let promise_group = promise_group_reference.get(agent).bind(gc.nogc());
        promise_group.get_mut(agent).remaining_elements_count += 1;
        let reaction = PromiseReactionHandler::PromiseGroup {
            index,
            promise_group,
        };

        // k. Set onFulfilled.[[Capability]] to resultCapability.
        // l. Set onFulfilled.[[RemainingElements]] to remainingElementsCount.
        // m. Set remainingElementsCount.[[Value]] to remainingElementsCount.[[Value]] + 1.
        // n. Perform ? Invoke(nextPromise, "then", « onFulfilled, resultCapability.[[Reject]] »).
        inner_promise_then(
            agent,
            next_promise.unbind(),
            reaction.unbind(),
            reaction.unbind(),
            None,
            gc.nogc(),
        );

        // o. Set index to index + 1.
        index += 1;
    }
}

fn throw_promise_subclassing_not_supported<'a>(
    agent: &mut Agent,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    agent.throw_exception_with_static_message(
        ExceptionType::Error,
        "Promise subclassing is not supported",
        gc,
    )
}
