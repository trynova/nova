// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::{Bindable, GcScope};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::invoke,
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin,
            promise::{
                Promise,
                data::{PromiseReactions, PromiseState},
            },
        },
        execution::{
            Agent, JsResult, RealmIdentifier,
            agent::{ExceptionType, PromiseRejectionTrackerOperation},
        },
        types::{BUILTIN_STRING_MEMORY, Function, IntoValue, String, Value},
    },
    heap::{CreateHeapData, WellKnownSymbolIndexes},
};

use super::promise_abstract_operations::{
    promise_capability_records::PromiseCapability,
    promise_jobs::new_promise_reaction_job,
    promise_reaction_records::{
        PromiseReactionHandler, PromiseReactionRecord, PromiseReactionType,
    },
};

pub(crate) struct PromisePrototype;

struct PromisePrototypeCatch;
impl Builtin for PromisePrototypeCatch {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.catch;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromisePrototype::catch);
}
struct PromisePrototypeFinally;
impl Builtin for PromisePrototypeFinally {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.finally;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromisePrototype::finally);
}
struct PromisePrototypeThen;
impl Builtin for PromisePrototypeThen {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.then;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(PromisePrototype::then);
}

impl PromisePrototype {
    fn catch<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let promise be the this value.
        // 2. Return ? Invoke(promise, "then", « undefined, onRejected »).
        // TODO: Add a fast path that calls `perform_promise_then` if we know
        // `this.then` is this realm's creation-time `Promise.prototype.then`.
        let on_rejected = args.get(0).unbind();
        invoke(
            agent,
            this_value,
            BUILTIN_STRING_MEMORY.then.into(),
            Some(ArgumentsList::from_mut_slice(&mut [
                Value::Undefined,
                on_rejected,
            ])),
            gc,
        )
    }

    fn finally<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!()
    }

    /// ### [27.2.5.4 Promise.prototype.then ( onFulfilled, onRejected )](https://tc39.es/ecma262/#sec-promise.prototype.then)
    fn then<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let gc = gc.into_nogc();
        let on_fulfilled = args.get(0).bind(gc);
        let on_rejected = args.get(1).bind(gc);
        // 1. Let promise be the this value.
        // 2. If IsPromise(promise) is false, throw a TypeError exception.
        let Value::Promise(promise) = this_value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not a promise",
                gc,
            ));
        };

        // 3. Let C be ? SpeciesConstructor(promise, %Promise%).
        // 4. Let resultCapability be ? NewPromiseCapability(C).
        // NOTE: We're ignoring species and subclasses.
        let result_capability = PromiseCapability::new(agent);

        // 5. Return PerformPromiseThen(promise, onFulfilled, onRejected, resultCapability).
        perform_promise_then(
            agent,
            promise,
            on_fulfilled,
            on_rejected,
            Some(result_capability),
        );
        Ok(result_capability.promise().into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.promise_prototype();
        let promise_constructor = intrinsics.promise();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(5)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<PromisePrototypeCatch>()
            .with_constructor_property(promise_constructor)
            .with_builtin_function_property::<PromisePrototypeFinally>()
            .with_builtin_function_property::<PromisePrototypeThen>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Promise.into_value())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

/// [27.2.5.4.1 PerformPromiseThen ( promise, onFulfilled, onRejected \[ , resultCapability \] )](https://tc39.es/ecma262/#sec-performpromisethen)
pub(crate) fn perform_promise_then(
    agent: &mut Agent,
    promise: Promise,
    on_fulfilled: Value,
    on_rejected: Value,
    result_capability: Option<PromiseCapability>,
) {
    // 3. If IsCallable(onFulfilled) is false, then
    //     a. Let onFulfilledJobCallback be empty.
    // 4. Else,
    //     a. Let onFulfilledJobCallback be HostMakeJobCallback(onFulfilled).
    // TODO: Add the HostMakeJobCallback host hook. Leaving it for later, since in implementations
    // other than browsers, [[HostDefined]] must be EMPTY.
    let on_fulfilled_job_callback = match Function::try_from(on_fulfilled) {
        Ok(callback) => PromiseReactionHandler::JobCallback(callback.unbind()),
        Err(_) => PromiseReactionHandler::Empty,
    };
    // 5. If IsCallable(onRejected) is false, then
    //     a. Let onRejectedJobCallback be empty.
    // 6. Else,
    //     a. Let onRejectedJobCallback be HostMakeJobCallback(onRejected).
    let on_rejected_job_callback = match Function::try_from(on_rejected) {
        Ok(callback) => PromiseReactionHandler::JobCallback(callback.unbind()),
        Err(_) => PromiseReactionHandler::Empty,
    };

    inner_promise_then(
        agent,
        promise,
        on_fulfilled_job_callback,
        on_rejected_job_callback,
        result_capability,
    )
}

/// Corresponds to PerformPromiseThen starting at step 7. Useful for Nova-internal promise reaction
/// handlers, without a JS function.
pub(crate) fn inner_promise_then(
    agent: &mut Agent,
    promise: Promise,
    on_fulfilled: PromiseReactionHandler,
    on_rejected: PromiseReactionHandler,
    result_capability: Option<PromiseCapability>,
) {
    // 7. Let fulfillReaction be the PromiseReaction Record { [[Capability]]: resultCapability, [[Type]]: fulfill, [[Handler]]: onFulfilledJobCallback }.
    let fulfill_reaction = agent.heap.create(PromiseReactionRecord {
        capability: result_capability,
        reaction_type: PromiseReactionType::Fulfill,
        handler: on_fulfilled,
    });
    // 8. Let rejectReaction be the PromiseReaction Record { [[Capability]]: resultCapability, [[Type]]: reject, [[Handler]]: onRejectedJobCallback }.
    let reject_reaction = agent.heap.create(PromiseReactionRecord {
        capability: result_capability,
        reaction_type: PromiseReactionType::Reject,
        handler: on_rejected,
    });

    match &mut agent[promise].promise_state {
        // 9. If promise.[[PromiseState]] is pending, then
        PromiseState::Pending {
            fulfill_reactions,
            reject_reactions,
            ..
        } => {
            // a. Append fulfillReaction to promise.[[PromiseFulfillReactions]].
            match fulfill_reactions {
                Some(PromiseReactions::Many(reaction_vec)) => reaction_vec.push(fulfill_reaction),
                Some(PromiseReactions::One(reaction)) => {
                    *fulfill_reactions =
                        Some(PromiseReactions::Many(vec![*reaction, fulfill_reaction]))
                }
                None => *fulfill_reactions = Some(PromiseReactions::One(fulfill_reaction)),
            };
            // b. Append rejectReaction to promise.[[PromiseRejectReactions]].
            match reject_reactions {
                Some(PromiseReactions::Many(reaction_vec)) => reaction_vec.push(reject_reaction),
                Some(PromiseReactions::One(reaction)) => {
                    *reject_reactions =
                        Some(PromiseReactions::Many(vec![*reaction, reject_reaction]))
                }
                None => *reject_reactions = Some(PromiseReactions::One(reject_reaction)),
            };
        }
        // 10. Else if promise.[[PromiseState]] is fulfilled, then
        PromiseState::Fulfilled { promise_result } => {
            let promise_result = *promise_result;
            // a. Let value be promise.[[PromiseResult]].
            // b. Let fulfillJob be NewPromiseReactionJob(fulfillReaction, value).
            let fulfill_job = new_promise_reaction_job(agent, fulfill_reaction, promise_result);
            // c. Perform HostEnqueuePromiseJob(fulfillJob.[[Job]], fulfillJob.[[Realm]]).
            agent.host_hooks.enqueue_promise_job(fulfill_job);
        }
        // 11. Else,
        PromiseState::Rejected {
            promise_result,
            is_handled,
        } => {
            let promise_result = *promise_result;
            // a. Assert: The value of promise.[[PromiseState]] is rejected.
            // b. Let reason be promise.[[PromiseResult]].
            // c. If promise.[[PromiseIsHandled]] is false, perform HostPromiseRejectionTracker(promise, "handle").
            if !*is_handled {
                // 12. Set promise.[[PromiseIsHandled]] to true.
                // NOTE: `is_handled` is tied to the agent's lifetime, so we need to use and drop
                // the mutable reference before calling into the host hook.
                *is_handled = true;

                agent
                    .host_hooks
                    .promise_rejection_tracker(promise, PromiseRejectionTrackerOperation::Handle);
            }
            // d. Let rejectJob be NewPromiseReactionJob(rejectReaction, reason).
            let reject_job = new_promise_reaction_job(agent, reject_reaction, promise_result);
            // e. Perform HostEnqueuePromiseJob(rejectJob.[[Job]], rejectJob.[[Realm]]).
            agent.host_hooks.enqueue_promise_job(reject_job);
        }
    }
}
