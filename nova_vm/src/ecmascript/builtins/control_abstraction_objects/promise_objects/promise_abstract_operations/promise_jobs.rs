// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [27.2.2 Promise Jobs](https://tc39.es/ecma262/#sec-promise-jobs)

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                create_iter_result_object, iterator_close_with_error,
            },
            operations_on_objects::{call_function, get_function_realm},
        },
        builtins::{ArgumentsList, promise::Promise},
        execution::{
            Agent, JsResult,
            agent::{InnerJob, Job, JsError},
        },
        scripts_and_modules::module::{
            import_get_module_namespace, link_and_evaluate,
            module_semantics::cyclic_module_records::{
                async_module_execution_fulfilled, async_module_execution_rejected,
            },
        },
        types::{Function, IntoValue, Object, Value},
    },
    engine::{
        Global,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::CreateHeapData,
};

use super::{
    promise_capability_records::PromiseCapability,
    promise_reaction_records::{PromiseReaction, PromiseReactionHandler, PromiseReactionType},
    promise_resolving_functions::{PromiseResolvingFunctionHeapData, PromiseResolvingFunctionType},
};

#[derive(Debug)]
pub(crate) struct PromiseResolveThenableJob {
    promise_to_resolve: Global<Promise<'static>>,
    thenable: Global<Object<'static>>,
    then: Global<Function<'static>>,
}
impl PromiseResolveThenableJob {
    pub(crate) fn run<'a>(self, agent: &mut Agent, mut gc: GcScope<'a, '_>) -> JsResult<'a, ()> {
        let Self {
            promise_to_resolve,
            thenable,
            then,
        } = self;
        // The following are substeps of point 1 in NewPromiseResolveThenableJob.
        // a. Let resolvingFunctions be CreateResolvingFunctions(promiseToResolve).
        // Note: We do not take the Promise from the Global yet. It must be taken
        // out later, lest we start leaking memory here.
        let promise = promise_to_resolve.get(agent, gc.nogc()).bind(gc.nogc());
        let promise_capability = PromiseCapability::from_promise(promise, false);
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

        // b. Let thenCallResult be Completion(HostCallJobCallback(then, thenable, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »)).
        // TODO: Add the HostCallJobCallback host hook. For now we're using its default
        // implementation, which is calling the thenable, since only browsers should use a different
        // implementation.
        let then = then.take(agent).bind(gc.nogc());
        let thenable = thenable.take(agent).bind(gc.nogc()).into_value();
        let then_call_result = call_function(
            agent,
            then.unbind(),
            thenable.unbind(),
            Some(ArgumentsList::from_mut_slice(&mut [
                resolve_function.unbind(),
                reject_function.unbind(),
            ])),
            gc.reborrow(),
        )
        .unbind()
        .bind(gc.nogc());

        // Note: Now we must take the Promise from the Global.
        let promise = promise_to_resolve.take(agent).bind(gc.nogc());
        let promise_capability = PromiseCapability::from_promise(promise, false);

        // c. If thenCallResult is an abrupt completion, then
        if let Err(err) = then_call_result {
            // i. Return ? Call(resolvingFunctions.[[Reject]], undefined, « thenCallResult.[[Value]] »).
            promise_capability.reject(agent, err.value(), gc.nogc());
        }
        // d. Return ? thenCallResult.
        Ok(())
    }
}

/// ### [27.2.2.2 NewPromiseResolveThenableJob ( promiseToResolve, thenable, then )](https://tc39.es/ecma262/#sec-newpromiseresolvethenablejob)
pub(crate) fn new_promise_resolve_thenable_job(
    agent: &mut Agent,
    promise_to_resolve: Promise,
    thenable: Object,
    then: Function,
    gc: NoGcScope,
) -> Job {
    // 2. Let getThenRealmResult be Completion(GetFunctionRealm(then.[[Callback]])).
    // 5. NOTE: thenRealm is never null. When then.[[Callback]] is a revoked Proxy and no code runs, thenRealm is used to create error objects.
    let then_realm = match get_function_realm(agent, then, gc) {
        // 3. If getThenRealmResult is a normal completion, let thenRealm be getThenRealmResult.[[Value]].
        Ok(realm) => realm,
        // 4. Else, let thenRealm be the current Realm Record.
        Err(_) => agent.current_realm(gc),
    };
    // 6. Return the Record { [[Job]]: job, [[Realm]]: thenRealm }.
    Job {
        realm: Some(then_realm.unbind()),
        inner: InnerJob::PromiseResolveThenable(PromiseResolveThenableJob {
            promise_to_resolve: Global::new(agent, promise_to_resolve.unbind()),
            thenable: Global::new(agent, thenable.unbind()),
            then: Global::new(agent, then.unbind()),
        }),
    }
}

#[derive(Debug)]
pub(crate) struct PromiseReactionJob {
    reaction: Global<PromiseReaction<'static>>,
    argument: Global<Value<'static>>,
}
impl PromiseReactionJob {
    pub(crate) fn run<'a>(self, agent: &mut Agent, mut gc: GcScope<'a, '_>) -> JsResult<'a, ()> {
        let Self { reaction, argument } = self;
        let reaction = reaction.take(agent).bind(gc.nogc());
        let argument = argument.take(agent).bind(gc.nogc());

        let (handler_result, promise_capability) = match agent[reaction].handler {
            PromiseReactionHandler::Empty => {
                let capability = agent[reaction].capability.clone().unwrap().bind(gc.nogc());
                match agent[reaction].reaction_type {
                    PromiseReactionType::Fulfill => {
                        // d.i.1. Let handlerResult be NormalCompletion(argument).
                        (Ok(argument), capability)
                    }
                    PromiseReactionType::Reject => {
                        // d.ii.1. Let handlerResult be ThrowCompletion(argument).
                        (Err(JsError::new(argument)), capability)
                    }
                }
            }
            // e.1. Let handlerResult be Completion(HostCallJobCallback(handler, undefined, « argument »)).
            // TODO: Add the HostCallJobCallback host hook. For now we're using its default
            // implementation, which is calling the thenable, since only browsers should use a
            // different implementation.
            PromiseReactionHandler::JobCallback(callback) => {
                let reaction = reaction.scope(agent, gc.nogc());
                let result = call_function(
                    agent,
                    callback.unbind(),
                    Value::Undefined,
                    Some(ArgumentsList::from_mut_value(&mut argument.unbind())),
                    gc.reborrow(),
                )
                .unbind()
                .bind(gc.nogc());
                // SAFETY: reaction is not shared.
                let reaction = unsafe { reaction.take(agent) };
                (
                    result,
                    agent[reaction].capability.clone().unwrap().bind(gc.nogc()),
                )
            }
            PromiseReactionHandler::Await(await_reaction) => {
                assert!(agent[reaction].capability.is_none());
                let reaction_type = agent[reaction].reaction_type;
                await_reaction.resume(agent, reaction_type, argument.unbind(), gc.reborrow());
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // 5. f. Return undefined.
                return Ok(());
            }
            PromiseReactionHandler::AsyncGenerator(async_generator) => {
                assert!(agent[reaction].capability.is_none());
                let reaction_type = agent[reaction].reaction_type;
                async_generator.resume_await(
                    agent,
                    reaction_type,
                    argument.unbind(),
                    gc.reborrow(),
                );
                return Ok(());
            }
            PromiseReactionHandler::AsyncFromSyncIterator { done } => {
                let capability = agent[reaction].capability.clone().unwrap().bind(gc.nogc());
                // 9. Let unwrap be a new Abstract Closure with parameters (v)
                //    that captures done and performs the following steps when
                //    called:
                // a. Return CreateIteratorResultObject(v, done).
                (
                    Ok(create_iter_result_object(agent, argument, done).into_value()),
                    capability,
                )
            }
            PromiseReactionHandler::AsyncFromSyncIteratorClose(object) => {
                let reaction = reaction.scope(agent, gc.nogc());
                // a. Let closeIterator be a new Abstract Closure with
                //    parameters (error) that captures syncIteratorRecord and
                //    performs the following steps when called:
                // i. Return ? IteratorClose(syncIteratorRecord, ThrowCompletion(error)).
                let err = JsError::new(argument);
                let err =
                    iterator_close_with_error(agent, object.unbind(), err.unbind(), gc.reborrow())
                        .unbind()
                        .bind(gc.nogc());
                // SAFETY: reaction is not shared.
                let reaction = unsafe { reaction.take(agent) }.bind(gc.nogc());
                let capability = agent[reaction].capability.clone().unwrap().bind(gc.nogc());
                (Err(err), capability)
            }
            PromiseReactionHandler::AsyncModule(module) => {
                assert!(agent[reaction].capability.is_none());
                match agent[reaction].reaction_type {
                    PromiseReactionType::Fulfill => {
                        // a. Perform AsyncModuleExecutionFulfilled(module).
                        async_module_execution_fulfilled(agent, module.unbind(), gc);
                    }
                    PromiseReactionType::Reject => {
                        let error = JsError::new(argument);
                        // a. Perform AsyncModuleExecutionRejected(module, error).
                        async_module_execution_rejected(
                            agent,
                            module.unbind(),
                            error.unbind(),
                            gc.into_nogc(),
                        );
                    }
                }
                // b. Return undefined.
                return Ok(());
            }
            PromiseReactionHandler::DynamicImport { promise, module } => {
                assert!(agent[reaction].capability.is_none());
                match agent[reaction].reaction_type {
                    PromiseReactionType::Fulfill => {
                        link_and_evaluate(agent, promise.unbind(), module.unbind(), gc);
                        return Ok(());
                    }
                    PromiseReactionType::Reject => {
                        // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « reason »).
                        // b. Return unused.
                        (
                            Err(JsError::new(argument)),
                            PromiseCapability::from_promise(promise, true),
                        )
                    }
                }
            }
            PromiseReactionHandler::DynamicImportEvaluate { promise, module } => {
                assert!(agent[reaction].capability.is_none());
                match agent[reaction].reaction_type {
                    PromiseReactionType::Fulfill => {
                        import_get_module_namespace(
                            agent,
                            promise.unbind(),
                            module.unbind(),
                            gc.into_nogc(),
                        );
                        return Ok(());
                    }
                    PromiseReactionType::Reject => {
                        // a. Perform ! Call(promiseCapability.[[Reject]], undefined, « reason »).
                        // b. Return unused.
                        (
                            Err(JsError::new(argument)),
                            PromiseCapability::from_promise(promise, true),
                        )
                    }
                }
            }
            PromiseReactionHandler::PromiseAll { promise_all, index } => {
                let capability = agent[reaction].capability.clone().unwrap().bind(gc.nogc());
                match agent[reaction].reaction_type {
                    PromiseReactionType::Fulfill => {
                        // Take out the record to drop the vec borrow before we use `agent`/`gc`
                        let rec = {
                            let slot = agent
                                .heap
                                .promise_all_records
                                .get_mut(promise_all.get_index())
                                .expect("PromiseAllRecord out of bounds");
                            slot.take().expect("PromiseAllRecord slot empty")
                        };

                        // Bind to current scope and mutate
                        let mut rec_bound = rec.unbind().bind(gc.nogc());
                        rec_bound.on_promise_fufilled(agent, index, argument.unbind(), gc.nogc());

                        // Write back with 'static lifetime
                        agent
                            .heap
                            .promise_all_records
                            .get_mut(promise_all.get_index())
                            .unwrap()
                            .replace(rec_bound.unbind());

                        (Ok(argument), capability)
                    }
                    PromiseReactionType::Reject => (Err(JsError::new(argument)), capability),
                }
            }
        };

        // f. If promiseCapability is undefined, then
        // i. Assert: handlerResult is not an abrupt completion.
        // ii. Return empty.

        match handler_result {
            // h. If handlerResult is an abrupt completion, then
            Err(err) => {
                // i. Return ? Call(promiseCapability.[[Reject]], undefined, « handlerResult.[[Value]] »).
                promise_capability.reject(agent, err.value(), gc.nogc())
            }
            // i. Else,
            Ok(value) => {
                // i. Return ? Call(promiseCapability.[[Resolve]], undefined, « handlerResult.[[Value]] »).
                promise_capability
                    .unbind()
                    .resolve(agent, value.unbind(), gc)
            }
        };
        Ok(())
    }
}

/// ### [27.2.2.1 NewPromiseReactionJob ( reaction, argument )](https://tc39.es/ecma262/#sec-newpromisereactionjob)
pub(crate) fn new_promise_reaction_job(
    agent: &mut Agent,
    reaction: PromiseReaction,
    argument: Value,
    gc: NoGcScope,
) -> Job {
    let handler_realm = match agent[reaction].handler {
        // 3. If reaction.[[Handler]] is not empty, then
        PromiseReactionHandler::JobCallback(callback) => {
            // a. Let getHandlerRealmResult be Completion(GetFunctionRealm(reaction.[[Handler]].[[Callback]])).
            // d. NOTE: handlerRealm is never null unless the handler is undefined. When the handler is a revoked Proxy and no ECMAScript code runs, handlerRealm is used to create error objects.
            match get_function_realm(agent, callback, gc) {
                // b. If getHandlerRealmResult is a normal completion, set handlerRealm to getHandlerRealmResult.[[Value]].
                Ok(realm) => Some(realm),
                // c. Else, set handlerRealm to the current Realm Record.
                Err(_) => Some(agent.current_realm(gc)),
            }
        }
        // In the spec, await continuations are JS functions created in the `Await()` spec
        // operation. Since `Await()` is called inside the execution context of the async function,
        // the realm of the continuation function is the same as the async function's realm.
        PromiseReactionHandler::Await(await_reaction) => Some(
            agent[await_reaction]
                .execution_context
                .as_ref()
                .unwrap()
                .realm,
        ),
        // 2. Let handlerRealm be null.
        PromiseReactionHandler::AsyncGenerator(_)
        | PromiseReactionHandler::Empty
        | PromiseReactionHandler::AsyncFromSyncIterator { .. }
        | PromiseReactionHandler::AsyncFromSyncIteratorClose(_)
        | PromiseReactionHandler::AsyncModule(_)
        | PromiseReactionHandler::DynamicImport { .. }
        | PromiseReactionHandler::DynamicImportEvaluate { .. }
        | PromiseReactionHandler::PromiseAll { .. } => None,
    };

    // 4. Return the Record { [[Job]]: job, [[Realm]]: handlerRealm }.
    let reaction = Global::new(agent, reaction.unbind());
    let argument = Global::new(agent, argument.unbind());
    Job {
        realm: handler_realm.unbind(),
        inner: InnerJob::PromiseReaction(PromiseReactionJob { reaction, argument }),
    }
}
