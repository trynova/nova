// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub(crate) use data::*;

use crate::{
    ecmascript::{
        Agent, InternalMethods, InternalSlots, JsError, JsResult, OrdinaryObject,
        PromiseCapability, ProtoIntrinsics, Value, object_handle,
    },
    engine::{
        Bindable, GcScope, NoGcScope,
        Scopable,
    },
    heap::{
        ArenaAccess, ArenaAccessMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, WorkQueues, arena_vec_access, BaseIndex,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Promise<'a>(BaseIndex<'a, PromiseHeapData<'static>>);
object_handle!(Promise);
arena_vec_access!(Promise, 'a, PromiseHeapData, promises);

impl<'a> Promise<'a> {
    /// Create a new resolved Promise.
    pub(crate) fn new_resolved(agent: &mut Agent, value: Value<'a>) -> Self {
        agent.heap.create(PromiseHeapData {
            object_index: None,
            promise_state: PromiseState::Fulfilled {
                promise_result: value,
            },
        })
    }

    /// Create a new rejected, unhandled Promise.
    pub(crate) fn new_rejected(agent: &mut Agent, error: Value, gc: NoGcScope<'a, '_>) -> Self {
        agent
            .heap
            .create(PromiseHeapData {
                object_index: None,
                promise_state: PromiseState::Rejected {
                    promise_result: error.unbind(),
                    is_handled: false,
                },
            })
            .bind(gc)
    }

    /// Get the result of a resolved Promise, or None if the Promise is not
    /// resolved.
    pub(crate) fn try_get_result<'gc>(
        self,
        agent: &Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<JsResult<'gc, Value<'gc>>> {
        match &self.get(agent).promise_state {
            PromiseState::Pending { .. } => None,
            PromiseState::Fulfilled { promise_result } => Some(Ok(promise_result.bind(gc))),
            PromiseState::Rejected { promise_result, .. } => {
                Some(Err(JsError::new(promise_result.bind(gc))))
            }
        }
    }

    pub(crate) fn set_already_resolved(self, agent: &mut Agent) {
        match &mut self.get_mut(agent).promise_state {
            PromiseState::Pending { is_resolved, .. } => *is_resolved = true,
            _ => unreachable!(),
        };
    }

    ///### [27.2.4.7.1 PromiseResolve ( C, x )](https://tc39.es/ecma262/#sec-promise-resolve)
    pub fn resolve(agent: &mut Agent, x: Value, mut gc: GcScope<'a, '_>) -> Self {
        // 1. If IsPromise(x) is true, then
        if let Value::Promise(promise) = x {
            // a. Let xConstructor be ? Get(x, "constructor").
            // b. If SameValue(xConstructor, C) is true, return x.
            // NOTE: Ignoring subclasses.
            promise.unbind()
        } else {
            // 2. Let promiseCapability be ? NewPromiseCapability(C).
            let promise_capability = PromiseCapability::new(agent, gc.nogc());
            let promise = promise_capability.promise().scope(agent, gc.nogc());
            // 3. Perform ? Call(promiseCapability.[[Resolve]], undefined, « x »).
            promise_capability.unbind().resolve(agent, x, gc.reborrow());
            // 4. Return promiseCapability.[[Promise]].
            // SAFETY: Not shared.
            unsafe { promise.take(agent) }
        }
    }
}

impl<'a> InternalSlots<'a> for Promise<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Promise;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for Promise<'a> {}

impl<'a> CreateHeapData<PromiseHeapData<'a>, Promise<'a>> for Heap {
    fn create(&mut self, data: PromiseHeapData<'a>) -> Promise<'a> {
        self.promises.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PromiseHeapData<'static>>();
        Promise(BaseIndex::last(&self.promises))
    }
}

impl HeapMarkAndSweep for Promise<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promises.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.promises.shift_index(&mut self.0)
    }
}

impl HeapSweepWeakReference for Promise<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.promises.shift_weak_index(self.0).map(Self)
    }
}
