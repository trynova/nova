// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [27.2.2 Promise Jobs](https://tc39.es/ecma262/#sec-promise-jobs)

use crate::engine::context::GcScope;
use crate::engine::Global;
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::{call_function, get_function_realm},
        builtins::{promise::Promise, ArgumentsList},
        execution::{
            agent::{InnerJob, Job, JsError},
            Agent, JsResult,
        },
        types::{Function, IntoValue, Object, Value},
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
    pub(crate) fn run(self, agent: &mut Agent, mut gc: GcScope) -> JsResult<()> {
        let Self {
            promise_to_resolve,
            thenable,
            then,
        } = self;
        // The following are substeps of point 1 in NewPromiseResolveThenableJob.
        // a. Let resolvingFunctions be CreateResolvingFunctions(promiseToResolve).
        let promise_capability =
            PromiseCapability::from_promise(promise_to_resolve.take(agent), false);
        let resolve_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability,
                resolve_type: PromiseResolvingFunctionType::Resolve,
            })
            .into_value();
        let reject_function = agent
            .heap
            .create(PromiseResolvingFunctionHeapData {
                object_index: None,
                promise_capability,
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
            Some(ArgumentsList(&[resolve_function, reject_function])),
            gc.reborrow(),
        );

        // c. If thenCallResult is an abrupt completion, then
        if let Err(err) = then_call_result {
            // i. Return ? Call(resolvingFunctions.[[Reject]], undefined, « thenCallResult.[[Value]] »).
            promise_capability.reject(agent, err.value());
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
) -> Job {
    // 2. Let getThenRealmResult be Completion(GetFunctionRealm(then.[[Callback]])).
    // 5. NOTE: thenRealm is never null. When then.[[Callback]] is a revoked Proxy and no code runs, thenRealm is used to create error objects.
    let then_realm = match get_function_realm(agent, then) {
        // 3. If getThenRealmResult is a normal completion, let thenRealm be getThenRealmResult.[[Value]].
        Ok(realm) => realm,
        // 4. Else, let thenRealm be the current Realm Record.
        Err(_) => agent.current_realm_id(),
    };
    // 6. Return the Record { [[Job]]: job, [[Realm]]: thenRealm }.
    Job {
        realm: Some(then_realm),
        inner: InnerJob::PromiseResolveThenable(PromiseResolveThenableJob {
            promise_to_resolve: Global::new(agent, promise_to_resolve.unbind()),
            thenable: Global::new(agent, thenable.unbind()),
            then: Global::new(agent, then.unbind()),
        }),
    }
}

#[derive(Debug)]
pub(crate) struct PromiseReactionJob {
    reaction: Global<PromiseReaction>,
    argument: Global<Value<'static>>,
}
impl PromiseReactionJob {
    pub(crate) fn run(self, agent: &mut Agent, mut gc: GcScope) -> JsResult<()> {
        let Self { reaction, argument } = self;
        let reaction = reaction.take(agent);
        let argument = argument.take(agent).bind(gc.nogc()).unbind();
        // The following are substeps of point 1 in NewPromiseReactionJob.
        let handler_result = match agent[reaction].handler {
            PromiseReactionHandler::Empty => match agent[reaction].reaction_type {
                PromiseReactionType::Fulfill => {
                    // d.i.1. Let handlerResult be NormalCompletion(argument).
                    Ok(argument)
                }
                PromiseReactionType::Reject => {
                    // d.ii.1. Let handlerResult be ThrowCompletion(argument).
                    Err(JsError::new(argument))
                }
            },
            // e.1. Let handlerResult be Completion(HostCallJobCallback(handler, undefined, « argument »)).
            // TODO: Add the HostCallJobCallback host hook. For now we're using its default
            // implementation, which is calling the thenable, since only browsers should use a
            // different implementation.
            PromiseReactionHandler::JobCallback(callback) => call_function(
                agent,
                callback,
                Value::Undefined,
                Some(ArgumentsList(&[argument])),
                gc.reborrow(),
            ),
            PromiseReactionHandler::Await(await_reaction) => {
                assert!(agent[reaction].capability.is_none());
                let reaction_type = agent[reaction].reaction_type;
                await_reaction.resume(agent, reaction_type, argument, gc.reborrow());
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // 5. f. Return undefined.
                Ok(Value::Undefined)
            }
            PromiseReactionHandler::AsyncGenerator(async_generator) => {
                assert!(agent[reaction].capability.is_none());
                let reaction_type = agent[reaction].reaction_type;
                async_generator.resume_await(agent, reaction_type, argument, gc.reborrow());
                Ok(Value::Undefined)
            }
        };

        // f. If promiseCapability is undefined, then
        let Some(promise_capability) = agent[reaction].capability else {
            // i. Assert: handlerResult is not an abrupt completion.
            handler_result.unwrap();
            // ii. Return empty.
            return Ok(());
        };
        match handler_result {
            // h. If handlerResult is an abrupt completion, then
            Err(err) => {
                // i. Return ? Call(promiseCapability.[[Reject]], undefined, « handlerResult.[[Value]] »).
                promise_capability.reject(agent, err.value())
            }
            // i. Else,
            Ok(value) => {
                // i. Return ? Call(promiseCapability.[[Resolve]], undefined, « handlerResult.[[Value]] »).
                promise_capability.resolve(agent, value.unbind(), gc)
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
) -> Job {
    let handler_realm = match agent[reaction].handler {
        // 3. If reaction.[[Handler]] is not empty, then
        PromiseReactionHandler::JobCallback(callback) => {
            // a. Let getHandlerRealmResult be Completion(GetFunctionRealm(reaction.[[Handler]].[[Callback]])).
            // d. NOTE: handlerRealm is never null unless the handler is undefined. When the handler is a revoked Proxy and no ECMAScript code runs, handlerRealm is used to create error objects.
            match get_function_realm(agent, callback) {
                // b. If getHandlerRealmResult is a normal completion, set handlerRealm to getHandlerRealmResult.[[Value]].
                Ok(realm) => Some(realm),
                // c. Else, set handlerRealm to the current Realm Record.
                Err(_) => Some(agent.current_realm_id()),
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
        PromiseReactionHandler::AsyncGenerator(_) | PromiseReactionHandler::Empty => None,
    };

    // 4. Return the Record { [[Job]]: job, [[Realm]]: handlerRealm }.
    let reaction = Global::new(agent, reaction);
    let argument = Global::new(agent, argument.unbind());
    Job {
        realm: handler_realm,
        inner: InnerJob::PromiseReaction(PromiseReactionJob { reaction, argument }),
    }
}
