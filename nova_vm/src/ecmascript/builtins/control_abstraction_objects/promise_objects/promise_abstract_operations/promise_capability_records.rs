// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [27.2.1.1 PromiseCapability Records](https://tc39.es/ecma262/#sec-promisecapability-records)

use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::{get, try_get},
        builtins::promise::{
            Promise,
            data::{PromiseHeapData, PromiseState},
        },
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, PromiseRejectionTrackerOperation, TryError, TryResult},
        },
        types::{BUILTIN_STRING_MEMORY, Function,  Object, TryGetResult, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::Scopable,
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PromiseCapability<'a> {
    pub(crate) promise: Promise<'a>,
    pub(crate) must_be_unresolved: bool,
}

impl<'a> PromiseCapability<'a> {
    ///### [27.2.1.5 NewPromiseCapability ( C )](https://tc39.es/ecma262/#sec-newpromisecapability)
    ///
    /// NOTE: Our implementation doesn't take C as a parameter, since we don't
    /// yet support promise subclassing.
    pub fn new(agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Self {
        Self::from_promise(agent.heap.create(PromiseHeapData::default()), true).bind(gc)
    }

    pub fn from_promise(promise: Promise<'a>, must_be_unresolved: bool) -> Self {
        Self {
            promise,
            must_be_unresolved,
        }
    }

    pub fn promise(&self) -> Promise<'a> {
        self.promise
    }

    fn is_already_resolved(&self, agent: &Agent) -> bool {
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

    ///### [27.2.1.4 FulfillPromise ( promise, value )](https://tc39.es/ecma262/#sec-fulfillpromise)
    pub(crate) fn internal_fulfill(&self, agent: &mut Agent, value: Value, gc: NoGcScope) {
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
            promise_result: value.unbind(),
        };
        // 7. Perform TriggerPromiseReactions(reactions, value)
        if let Some(reactions) = reactions {
            reactions.trigger(agent, value, gc);
        }
    }

    ///### [27.2.1.7 RejectPromise ( promise, reason )](https://tc39.es/ecma262/#sec-rejectpromise)
    fn internal_reject(&self, agent: &mut Agent, reason: Value, gc: NoGcScope) {
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
            promise_result: reason.unbind(),
            is_handled: reactions.is_some(),
        };

        // 7. If promise.[[PromiseIsHandled]] is false, perform HostPromiseRejectionTracker(promise, "reject").
        agent
            .host_hooks
            .promise_rejection_tracker(self.promise, PromiseRejectionTrackerOperation::Reject);

        // 8. Perform TriggerPromiseReactions(reactions, reason)
        if let Some(reactions) = reactions {
            reactions.trigger(agent, reason, gc);
        }
    }

    ///### [27.2.1.3.2 Promise Resolve Functions](https://tc39.es/ecma262/#sec-promise-resolve-functions)
    pub fn resolve(self, agent: &mut Agent, resolution: Value, mut gc: GcScope) {
        let promise_capability = self.bind(gc.nogc());
        let resolution = resolution.bind(gc.nogc());
        // 1. Let F be the active function object.
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if promise_capability.is_already_resolved(agent) {
            return;
        }
        let PromiseCapability {
            promise,
            must_be_unresolved,
        } = promise_capability;
        let promise = promise.bind(gc.nogc());
        // 6. Set alreadyResolved.[[Value]] to true.
        promise.set_already_resolved(agent);

        // 7. If SameValue(resolution, promise) is true, then
        if resolution == promise.into() {
            // a. Let selfResolutionError be a newly created TypeError object.
            // b. Perform RejectPromise(promise, selfResolutionError).
            let exception = agent
                .create_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Tried to resolve a promise with itself.",
                    gc.nogc(),
                )
                .unbind();
            promise_capability.internal_reject(agent, exception, gc.nogc());
            // c. Return undefined.
            return;
        }

        // 8. If resolution is not an Object, then
        let Ok(resolution) = Object::try_from(resolution) else {
            // a. Perform FulfillPromise(promise, resolution).
            promise_capability.internal_fulfill(agent, resolution, gc.nogc());
            // b. Return undefined.
            return;
        };

        let promise = promise.scope(agent, gc.nogc());
        let scoped_resolution = resolution.scope(agent, gc.nogc());
        // 9. Let then be Completion(Get(resolution, "then")).
        let then_action = match get(
            agent,
            resolution.unbind(),
            BUILTIN_STRING_MEMORY.then.into(),
            gc.reborrow(),
        ) {
            // 11. Let thenAction be then.[[Value]].
            Ok(then_action) => then_action.unbind().bind(gc.nogc()),
            // 10. If then is an abrupt completion, then
            Err(err) => {
                // a. Perform RejectPromise(promise, then.[[Value]]).
                PromiseCapability {
                    promise: promise.get(agent),
                    must_be_unresolved,
                }
                .internal_reject(agent, err.value().unbind(), gc.nogc());
                // b. Return undefined.
                return;
            }
        };

        let resolution = scoped_resolution.get(agent).bind(gc.nogc());
        // 12. If IsCallable(thenAction) is false, then
        // TODO: Callable proxies
        let Ok(then_action) = Function::try_from(then_action) else {
            // a. Perform FulfillPromise(promise, resolution).
            PromiseCapability {
                promise: promise.get(agent),
                must_be_unresolved,
            }
            .internal_fulfill(agent, resolution.into().unbind(), gc.nogc());
            // b. Return undefined.
            return;
        };
        // SAFETY: Promise is not shared.
        let promise = unsafe { promise.take(agent) }.bind(gc.nogc());

        // 13. Let thenJobCallback be HostMakeJobCallback(thenAction).
        // TODO: Add the HostMakeJobCallback host hook. Leaving it for later, since in
        // implementations other than browsers, [[HostDefine]] must be EMPTY.
        // 14. Let job be NewPromiseResolveThenableJob(promise, resolution, thenJobCallback).
        let job = new_promise_resolve_thenable_job(
            agent,
            promise.unbind(),
            resolution.unbind(),
            then_action.unbind(),
            gc.into_nogc(),
        );
        // 15. Perform HostEnqueuePromiseJob(job.[[Job]], job.[[Realm]]).
        agent.host_hooks.enqueue_promise_job(job);
        // 16. Return undefined.
    }

    ///### [27.2.1.3.2 Promise Resolve Functions](https://tc39.es/ecma262/#sec-promise-resolve-functions)
    pub fn try_resolve<'gc>(
        &self,
        agent: &mut Agent,
        resolution: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, ()> {
        // 1. Let F be the active function object.
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if self.is_already_resolved(agent) {
            return TryResult::Continue(());
        }
        // 6. Set alreadyResolved.[[Value]] to true.
        match &mut agent[self.promise].promise_state {
            PromiseState::Pending { is_resolved, .. } => *is_resolved = true,
            _ => unreachable!(),
        };

        // 7. If SameValue(resolution, promise) is true, then
        if resolution == self.promise.into() {
            // a. Let selfResolutionError be a newly created TypeError object.
            // b. Perform RejectPromise(promise, selfResolutionError).
            let exception = agent
                .create_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Tried to resolve a promise with itself.",
                    gc,
                )
                .unbind();
            self.internal_reject(agent, exception, gc);
            // c. Return undefined.
            return TryResult::Continue(());
        }

        // 8. If resolution is not an Object, then
        let Ok(resolution) = Object::try_from(resolution) else {
            // a. Perform FulfillPromise(promise, resolution).
            self.internal_fulfill(agent, resolution, gc);
            // b. Return undefined.
            return TryResult::Continue(());
        };

        // 9. Let then be Completion(Get(resolution, "then")).
        // 10. If then is an abrupt completion, then
        // a. Perform RejectPromise(promise, then.[[Value]]).
        // b. Return undefined.
        // 11. Let thenAction be then.[[Value]].
        let then_action = match try_get(
            agent,
            resolution,
            BUILTIN_STRING_MEMORY.then.into(),
            None,
            gc,
        )? {
            TryGetResult::Unset => Value::Undefined,
            TryGetResult::Value(v) => v,
            _ => return TryError::GcError.into(),
        };

        // 12. If IsCallable(thenAction) is false, then
        // TODO: Callable proxies
        let Ok(then_action) = Function::try_from(then_action) else {
            // a. Perform FulfillPromise(promise, resolution).
            self.internal_fulfill(agent, resolution.into(), gc);
            // b. Return undefined.
            return TryResult::Continue(());
        };

        // 13. Let thenJobCallback be HostMakeJobCallback(thenAction).
        // TODO: Add the HostMakeJobCallback host hook. Leaving it for later, since in
        // implementations other than browsers, [[HostDefine]] must be EMPTY.
        // 14. Let job be NewPromiseResolveThenableJob(promise, resolution, thenJobCallback).
        let job =
            new_promise_resolve_thenable_job(agent, self.promise, resolution, then_action, gc);
        // 15. Perform HostEnqueuePromiseJob(job.[[Job]], job.[[Realm]]).
        agent.host_hooks.enqueue_promise_job(job);
        // 16. Return undefined.
        TryResult::Continue(())
    }

    ///### [27.2.1.3.1 Promise Reject Functions](https://tc39.es/ecma262/#sec-promise-reject-functions)
    pub fn reject(&self, agent: &mut Agent, reason: Value, gc: NoGcScope) {
        // 1. Let F be the active function object.
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if self.is_already_resolved(agent) {
            return;
        }

        let promise = self.promise();

        // 7. Perform RejectPromise(promise, reason).
        self.internal_reject(agent, reason, gc);

        // 6. Set alreadyResolved.[[Value]] to true.
        debug_assert!(matches!(
            agent[promise].promise_state,
            PromiseState::Rejected { .. }
        ));
    }
}

bindable_handle!(PromiseCapability);

impl HeapMarkAndSweep for PromiseCapability<'static> {
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
pub(crate) fn if_abrupt_reject_promise<'gc, T: 'gc>(
    agent: &mut Agent,
    value: JsResult<T>,
    capability: PromiseCapability,
    gc: NoGcScope<'gc, '_>,
) -> Result<T, Promise<'gc>> {
    value.map_err(|err| {
        let promise = capability.promise().bind(gc);
        capability.reject(agent, err.value(), gc);

        // Note: We return an error here so that caller gets to call this
        // function with the ? operator
        promise
    })
}

macro_rules! if_abrupt_reject_promise_m {
    ($agent:ident, $value:ident, $capability:ident, $gc:ident) => {
        // 1. Assert: value is a Completion Record.
        match $value.unbind().bind($gc.nogc()) {
            // 2. If value is an abrupt completion, then
            Err(err) => {
                // a. Perform ? Call(capability.[[Reject]], undefined, « value.[[Value]] »).
                $capability.reject($agent, err.value().unbind(), $gc.nogc());
                // b. Return capability.[[Promise]].
                return $capability.promise.unbind().bind($gc.into_nogc()).into();
            }
            // 3. Else,
            Ok(value) => {
                // a. Set value to ! value.
                value.unbind().bind($gc.nogc())
            }
        }
    };
}

pub(crate) use if_abrupt_reject_promise_m;
