// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [27.2.2 Promise Jobs](https://tc39.es/ecma262/#sec-promise-jobs)

use crate::{
    ecmascript::{
        Agent, ArgumentsList, Function, InnerJob, Job, JsError, JsResult, Object, Promise,
        PromiseCapability, PromiseReaction, PromiseReactionHandler, PromiseReactionType,
        PromiseResolvingFunctionHeapData, PromiseResolvingFunctionType, Value,
        async_module_execution_fulfilled, async_module_execution_rejected, call_function,
        create_iter_result_object, get_function_realm, import_get_module_namespace,
        iterator_close_with_error, link_and_evaluate,
    },
    engine::{Bindable, GcScope, Global, NoGcScope, Scopable},
    heap::{ArenaAccess, CreateHeapData},
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
        crate::engine::bind!(let promise = promise_to_resolve.get(agent, gc.nogc()), gc);
        let promise_capability = PromiseCapability::from_promise(promise, false);
        let resolve_function = agent.heap.create(PromiseResolvingFunctionHeapData {
            object_index: None,
            promise_capability: promise_capability.clone(),
            resolve_type: PromiseResolvingFunctionType::Resolve,
        });
        let reject_function = agent.heap.create(PromiseResolvingFunctionHeapData {
            object_index: None,
            promise_capability: promise_capability.clone(),
            resolve_type: PromiseResolvingFunctionType::Reject,
        });

        // b. Let thenCallResult be Completion(HostCallJobCallback(then, thenable, « resolvingFunctions.[[Resolve]], resolvingFunctions.[[Reject]] »)).
        // TODO: Add the HostCallJobCallback host hook. For now we're using its default
        // implementation, which is calling the thenable, since only browsers should use a different
        // implementation.
        crate::engine::bind!(let then = then.take(agent).local(), gc);
        crate::engine::bind!(let thenable = thenable.take(agent).local(), gc);
        let then_call_result = call_function(
            agent,
            then,
            thenable.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                resolve_function.into(),
                reject_function.into(),
            ])),
            gc.reborrow(),
        )?;

        // Note: Now we must take the Promise from the Global.
        crate::engine::bind!(let promise = promise_to_resolve.take(agent).local(), gc);
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
        realm: Some(then_realm),
        inner: InnerJob::PromiseResolveThenable(PromiseResolveThenableJob {
            promise_to_resolve: Global::new(agent, promise_to_resolve),
            thenable: Global::new(agent, thenable),
            then: Global::new(agent, then),
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
        crate::engine::bind!(let reaction = reaction.take(agent).local(), gc);
        crate::engine::bind!(let argument = argument.take(agent).local(), gc);

        let reaction_data = reaction.get(agent).local();

        let (handler_result, promise_capability) = match reaction_data.handler {
            PromiseReactionHandler::Empty => {
                let capability = reaction_data.capability.clone().unwrap();
                match reaction_data.reaction_type {
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
                    callback,
                    Value::Undefined,
                    Some(ArgumentsList::from_mut_value(&mut argument)),
                    gc.reborrow(),
                )?;
                // SAFETY: reaction is not shared.
                let reaction = unsafe { reaction.take(agent).local() };
                (
                    result,
                    reaction.get(agent).local().capability.clone().unwrap(),
                )
            }
            PromiseReactionHandler::Await(await_reaction) => {
                assert!(reaction_data.capability.is_none());
                let reaction_type = reaction_data.reaction_type;
                await_reaction.resume(agent, reaction_type, argument, gc.reborrow());
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // 5. f. Return undefined.
                return Ok(());
            }
            PromiseReactionHandler::AsyncGenerator(async_generator) => {
                assert!(reaction_data.capability.is_none());
                let reaction_type = reaction_data.reaction_type;
                async_generator.resume_await(agent, reaction_type, argument, gc.reborrow());
                return Ok(());
            }
            PromiseReactionHandler::AsyncFromSyncIterator { done } => {
                crate::engine::bind!(let capability = reaction_data.capability.clone().unwrap(), gc);
                // 9. Let unwrap be a new Abstract Closure with parameters (v)
                //    that captures done and performs the following steps when
                //    called:
                // a. Return CreateIteratorResultObject(v, done).
                (
                    create_iter_result_object(agent, argument, done, gc.nogc()).map(|o| o.into()),
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
                let err = iterator_close_with_error(agent, object, err, gc.reborrow())?;
                // SAFETY: reaction is not shared.
                crate::engine::bind!(let reaction = unsafe { reaction.take(agent).local() }, gc);
                let capability = reaction.get(agent).local().capability.clone().unwrap();
                (Err(err), capability)
            }
            PromiseReactionHandler::AsyncModule(module) => {
                assert!(reaction_data.capability.is_none());
                match reaction_data.reaction_type {
                    PromiseReactionType::Fulfill => {
                        // a. Perform AsyncModuleExecutionFulfilled(module).
                        async_module_execution_fulfilled(agent, module, gc);
                    }
                    PromiseReactionType::Reject => {
                        let error = JsError::new(argument);
                        // a. Perform AsyncModuleExecutionRejected(module, error).
                        async_module_execution_rejected(agent, module, error, gc.into_nogc());
                    }
                }
                // b. Return undefined.
                return Ok(());
            }
            PromiseReactionHandler::DynamicImport { promise, module } => {
                assert!(reaction_data.capability.is_none());
                match reaction_data.reaction_type {
                    PromiseReactionType::Fulfill => {
                        link_and_evaluate(agent, promise, module, gc);
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
                assert!(reaction_data.capability.is_none());
                match reaction_data.reaction_type {
                    PromiseReactionType::Fulfill => {
                        import_get_module_namespace(agent, promise, module, gc.into_nogc());
                        return Ok(());
                    }
                    PromiseReactionType::Reject => (
                        Err(JsError::new(argument)),
                        PromiseCapability::from_promise(promise, true),
                    ),
                }
            }
            PromiseReactionHandler::PromiseGroup {
                promise_group,
                index,
            } => {
                let reaction_type = reaction_data.reaction_type;
                promise_group.settle(agent, reaction_type, index, argument, gc.reborrow());
                return Ok(());
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
                promise_capability.resolve(agent, value, gc)
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
    let handler_realm = match reaction.get(agent).local().handler {
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
            await_reaction
                .get(agent)
                .local()
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
        | PromiseReactionHandler::PromiseGroup { .. } => None,
    };

    // 4. Return the Record { [[Job]]: job, [[Realm]]: handlerRealm }.
    let reaction = Global::new(agent, reaction);
    let argument = Global::new(agent, argument);
    Job {
        realm: handler_realm,
        inner: InnerJob::PromiseReaction(PromiseReactionJob { reaction, argument }),
    }
}
