// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [27.2.1.1 PromiseCapability Records]()

use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::get,
        builtins::promise::{
            data::{PromiseHeapData, PromiseState},
            Promise,
        },
        execution::{
            agent::{ExceptionType, JsError, PromiseRejectionTrackerOperation},
            Agent, JsResult,
        },
        types::{Function, IntoValue, Object, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{CompactionLists, CreateHeapData, HeapMarkAndSweep, WorkQueues},
};

use super::promise_jobs::new_promise_resolve_thenable_job;

/// A promise capability encapsulates a promise, adding methods that are capable
/// of resolving or rejecting that promise.
///
/// NOTE: In the spec, promise capability records contain an object that is
/// usable as a promise, together with its resolve and reject functions. In our
/// current implementation, we only ever support built-in promises, and not
/// other promise-like objects (e.g. we don't support Promise subclasses), and
/// for that we don't need to store resolve and reject functions, we can create
/// them only when needed.
///
/// The `must_be_unresolved` boolean is used to map the `AlreadyResolved` state
/// of a pair of resolve/reject functions with the promise state. If
/// `must_be_unresolved` is false, the promise counts as already resolved if its
/// state is Fulfilled or Rejected. If true, it also counts as already resolved
/// if it's Pending but `is_resolved` is set to true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PromiseCapability {
    promise: Promise,
    must_be_unresolved: bool,
}

impl PromiseCapability {
    /// [27.2.1.5 NewPromiseCapability ( C )](https://tc39.es/ecma262/#sec-newpromisecapability)
    /// NOTE: Our implementation doesn't take C as a parameter, since we don't
    /// yet support promise subclassing.
    pub fn new(agent: &mut Agent) -> Self {
        Self::from_promise(agent.heap.create(PromiseHeapData::default()), true)
    }

    pub fn from_promise(promise: Promise, must_be_unresolved: bool) -> Self {
        Self {
            promise,
            must_be_unresolved,
        }
    }

    pub fn promise(&self) -> Promise {
        self.promise
    }

    fn is_already_resolved(self, agent: &mut Agent) -> bool {
        // If `self.must_be_unresolved` is true, then `alreadyResolved`
        // corresponds with the `is_resolved` flag in PromiseState::Pending.
        // Otherwise, it corresponds to `promise_state` not being Pending.
        match agent[self.promise].promise_state {
            PromiseState::Pending { is_resolved, .. } => {
                if self.must_be_unresolved {
                    is_resolved
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    /// [27.2.1.4 FulfillPromise ( promise, value )](https://tc39.es/ecma262/#sec-fulfillpromise)
    fn internal_fulfill(self, agent: &mut Agent, value: Value) {
        // 1. Assert: The value of promise.[[PromiseState]] is pending.
        // 2. Let reactions be promise.[[PromiseFulfillReactions]].
        let promise_state = &mut agent[self.promise].promise_state;
        let reactions = match promise_state {
            PromiseState::Pending {
                fulfill_reactions, ..
            } => fulfill_reactions.take(),
            _ => unreachable!(),
        };
        // 3. Set promise.[[PromiseResult]] to value.
        // 4. Set promise.[[PromiseFulfillReactions]] to undefined.
        // 5. Set promise.[[PromiseRejectReactions]] to undefined.
        // 6. Set promise.[[PromiseState]] to FULFILLED.
        *promise_state = PromiseState::Fulfilled {
            promise_result: value,
        };
        // 7. Perform TriggerPromiseReactions(reactions, value)
        if let Some(reactions) = reactions {
            reactions.trigger(agent, value);
        }
    }

    /// [27.2.1.7 RejectPromise ( promise, reason )](https://tc39.es/ecma262/#sec-rejectpromise)
    fn internal_reject(self, agent: &mut Agent, reason: Value) {
        // 1. Assert: The value of promise.[[PromiseState]] is pending.
        // 2. Let reactions be promise.[[PromiseRejectReactions]].
        let promise_state = &mut agent[self.promise].promise_state;
        let reactions = match promise_state {
            PromiseState::Pending {
                reject_reactions, ..
            } => reject_reactions.take(),
            _ => unreachable!(),
        };
        // 3. Set promise.[[PromiseResult]] to reason.
        // 4. Set promise.[[PromiseFulfillReactions]] to undefined.
        // 5. Set promise.[[PromiseRejectReactions]] to undefined.
        // 6. Set promise.[[PromiseState]] to REJECTED.
        // NOTE: [[PromiseIsHandled]] for pending promises corresponds to
        // whether [[PromiseRejectReactions]] is not empty.
        *promise_state = PromiseState::Rejected {
            promise_result: reason,
            is_handled: reactions.is_some(),
        };

        // 7. If promise.[[PromiseIsHandled]] is false, perform HostPromiseRejectionTracker(promise, "reject").
        agent
            .host_hooks
            .promise_rejection_tracker(self.promise, PromiseRejectionTrackerOperation::Reject);

        // 8. Perform TriggerPromiseReactions(reactions, reason)
        if let Some(reactions) = reactions {
            reactions.trigger(agent, reason);
        }
    }

    /// [27.2.1.3.2 Promise Resolve Functions](https://tc39.es/ecma262/#sec-promise-resolve-functions)
    pub fn resolve(self, agent: &mut Agent, resolution: Value) {
        // 1. Let F be the active function object.
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if self.is_already_resolved(agent) {
            return;
        }
        // 6. Set alreadyResolved.[[Value]] to true.
        match &mut agent[self.promise].promise_state {
            PromiseState::Pending { is_resolved, .. } => *is_resolved = true,
            _ => unreachable!(),
        };

        // 7. If SameValue(resolution, promise) is true, then
        if resolution == self.promise.into_value() {
            // a. Let selfResolutionError be a newly created TypeError object.
            // b. Perform RejectPromise(promise, selfResolutionError).
            let exception = agent.create_exception_with_static_message(
                ExceptionType::TypeError,
                "Tried to resolve a promise with itself.",
            );
            self.internal_reject(agent, exception);
            // c. Return undefined.
            return;
        }

        // 8. If resolution is not an Object, then
        let Ok(resolution) = Object::try_from(resolution) else {
            // a. Perform FulfillPromise(promise, resolution).
            self.internal_fulfill(agent, resolution);
            // b. Return undefined.
            return;
        };

        // 9. Let then be Completion(Get(resolution, "then")).
        let then_action = match get(agent, resolution, BUILTIN_STRING_MEMORY.then.into()) {
            // 11. Let thenAction be then.[[Value]].
            Ok(then_action) => then_action,
            // 10. If then is an abrupt completion, then
            Err(err) => {
                // a. Perform RejectPromise(promise, then.[[Value]]).
                self.internal_reject(agent, err.value());
                // b. Return undefined.
                return;
            }
        };

        // 12. If IsCallable(thenAction) is false, then
        // TODO: Callable proxies
        let Ok(then_action) = Function::try_from(then_action) else {
            // a. Perform FulfillPromise(promise, resolution).
            self.internal_fulfill(agent, resolution.into_value());
            // b. Return undefined.
            return;
        };

        // 13. Let thenJobCallback be HostMakeJobCallback(thenAction).
        // TODO: Add the HostMakeJobCallback host hook. Leaving it for later, since in
        // implementations other than browsers, [[HostDefine]] must be EMPTY.
        // 14. Let job be NewPromiseResolveThenableJob(promise, resolution, thenJobCallback).
        let job = new_promise_resolve_thenable_job(agent, self.promise, resolution, then_action);
        // 15. Perform HostEnqueuePromiseJob(job.[[Job]], job.[[Realm]]).
        agent.host_hooks.enqueue_promise_job(job);
        // 16. Return undefined.
    }

    /// [27.2.1.3.1 Promise Reject Functions](https://tc39.es/ecma262/#sec-promise-reject-functions)
    pub fn reject(self, agent: &mut Agent, reason: Value) {
        // 1. Let F be the active function object.
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if self.is_already_resolved(agent) {
            return;
        }

        // 7. Perform RejectPromise(promise, reason).
        self.internal_reject(agent, reason);

        // 6. Set alreadyResolved.[[Value]] to true.
        debug_assert!(matches!(
            agent[self.promise].promise_state,
            PromiseState::Rejected { .. }
        ));
    }
}

impl HeapMarkAndSweep for PromiseCapability {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            promise,
            must_be_unresolved: _,
        } = self;
        promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            promise,
            must_be_unresolved: _,
        } = self;
        promise.sweep_values(compactions);
    }
}

/// ### [27.2.1.1.1 IfAbruptRejectPromise ( value, capability )](https://tc39.es/ecma262/#sec-ifabruptrejectpromise)
///
/// IfAbruptRejectPromise is a shorthand for a sequence of algorithm steps that
/// use a PromiseCapability Record. An algorithm step of the form:
///
/// ```text
/// 1. IfAbruptRejectPromise(value, capability).
/// ```
///
/// means the same thing as:
/// ```text
/// 1. Assert: value is a Completion Record.
/// 2. If value is an abrupt completion, then
///     a. Perform ? Call(capability.[[Reject]], undefined, « value.[[Value]] »).
///     b. Return capability.[[Promise]].
/// 3. Else,
///     a. Set value to ! value.
/// ```
#[inline(always)]
pub(crate) fn if_abrupt_reject_promise<T>(
    agent: &mut Agent,
    value: JsResult<T>,
    capability: PromiseCapability,
) -> JsResult<T> {
    value.map_err(|err| {
        capability.reject(agent, err.value());

        // Note: We return an error here so that caller gets to call this
        // function with the ? operator
        JsError::new(capability.promise().into_value())
    })
}
