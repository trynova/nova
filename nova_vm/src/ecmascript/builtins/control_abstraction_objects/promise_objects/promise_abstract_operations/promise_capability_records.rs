// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [27.2.1.1 PromiseCapability Records]()

use crate::{
    ecmascript::{
        builtins::promise::Promise,
        execution::{agent::JsError, Agent, JsResult},
        types::{IntoValue, Value},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

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
pub(crate) struct PromiseCapability {
    promise: Promise,
    must_be_unresolved: bool,
}

impl PromiseCapability {
    /// [27.2.1.5 NewPromiseCapability ( C )](https://tc39.es/ecma262/#sec-newpromisecapability)
    pub(crate) fn new(_agent: &mut Agent) -> Self {
        todo!()
    }

    pub(crate) fn promise(&self) -> Promise {
        self.promise
    }

    /// [27.2.1.3.2 Promise Resolve Functions](https://tc39.es/ecma262/#sec-promise-resolve-functions)
    pub(crate) fn resolve(&self, _agent: &mut Agent, _resolution: Value) {
        todo!()
    }

    /// [27.2.1.3.1 Promise Reject Functions](https://tc39.es/ecma262/#sec-promise-reject-functions)
    pub(crate) fn reject(&self, _agent: &mut Agent, _reason: Value) {
        todo!()
    }
}

impl HeapMarkAndSweep for PromiseCapability {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.promise.sweep_values(compactions);
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
