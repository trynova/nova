// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [27.2.2 Promise Jobs](https://tc39.es/ecma262/#sec-promise-jobs)

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

#[derive(Debug, Clone, Copy)]
pub(crate) struct PromiseResolveThenableJob<'gen> {
    promise_to_resolve: Promise<'gen>,
    thenable: Object<'gen>,
    then: Function<'gen>,
}
impl<'gen> PromiseResolveThenableJob<'gen> {
    pub(crate) fn run(self, agent: &mut Agent<'gen>) -> JsResult<'gen, ()> {
        // The following are substeps of point 1 in NewPromiseResolveThenableJob.
        // a. Let resolvingFunctions be CreateResolvingFunctions(promiseToResolve).
        let promise_capability = PromiseCapability::from_promise(self.promise_to_resolve, false);
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
        let then_call_result = call_function(
            agent,
            self.then,
            self.thenable.into_value(),
            Some(ArgumentsList(&[resolve_function, reject_function])),
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
pub(crate) fn new_promise_resolve_thenable_job<'gen>(
    agent: &mut Agent<'gen>,
    promise_to_resolve: Promise<'gen>,
    thenable: Object<'gen>,
    then: Function<'gen>,
) -> Job<'gen> {
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
            promise_to_resolve,
            thenable,
            then,
        }),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PromiseReactionJob<'gen> {
    reaction: PromiseReaction<'gen>,
    argument: Value<'gen>,
}
impl<'gen> PromiseReactionJob<'gen> {
    pub(crate) fn run(self, agent: &mut Agent<'gen>) -> JsResult<'gen, ()> {
        // The following are substeps of point 1 in NewPromiseReactionJob.
        let handler_result = match agent[self.reaction].handler {
            PromiseReactionHandler::Empty => match agent[self.reaction].reaction_type {
                PromiseReactionType::Fulfill => {
                    // d.i.1. Let handlerResult be NormalCompletion(argument).
                    Ok(self.argument)
                }
                PromiseReactionType::Reject => {
                    // d.ii.1. Let handlerResult be ThrowCompletion(argument).
                    Err(JsError::new(self.argument))
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
                Some(ArgumentsList(&[self.argument])),
            ),
            PromiseReactionHandler::Await(await_reaction) => {
                assert!(agent[self.reaction].capability.is_none());
                let reaction_type = agent[self.reaction].reaction_type;
                await_reaction.resume(agent, reaction_type, self.argument);
                // [27.7.5.3 Await ( value )](https://tc39.es/ecma262/#await)
                // 3. f. Return undefined.
                // 5. f. Return undefined.
                Ok(Value::Undefined)
            }
        };

        // f. If promiseCapability is undefined, then
        let Some(promise_capability) = agent[self.reaction].capability else {
            // i. Assert: handlerResult is not an abrupt completion.
            handler_result.unwrap();
            // ii. Return empty.
            return Ok(());
        };
        match handler_result {
            // h. If handlerResult is an abrupt completion, then
            Err(err) => {
                // i. Return ? Call(promiseCapability.[[Resolve]], undefined, « handlerResult.[[Value]] »).
                promise_capability.reject(agent, err.value())
            }
            // i. Else,
            Ok(value) => {
                // i. Return ? Call(promiseCapability.[[Reject]], undefined, « handlerResult.[[Value]] »).
                promise_capability.resolve(agent, value)
            }
        };
        Ok(())
    }
}

/// ### [27.2.2.1 NewPromiseReactionJob ( reaction, argument )](https://tc39.es/ecma262/#sec-newpromisereactionjob)
pub(crate) fn new_promise_reaction_job<'gen>(
    agent: &mut Agent<'gen>,
    reaction: PromiseReaction<'gen>,
    argument: Value<'gen>,
) -> Job<'gen> {
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
        PromiseReactionHandler::Empty => None,
    };

    // 4. Return the Record { [[Job]]: job, [[Realm]]: handlerRealm }.
    Job {
        realm: handler_realm,
        inner: InnerJob::PromiseReaction(PromiseReactionJob { reaction, argument }),
    }
}
