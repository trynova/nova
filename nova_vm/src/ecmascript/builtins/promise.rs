// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, PromiseIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::PromiseHeapData;

use super::control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Promise(pub(crate) PromiseIndex);

impl Promise {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// [27.2.4.7.1 PromiseResolve ( C, x )](https://tc39.es/ecma262/#sec-promise-resolve)
    pub fn resolve(agent: &mut Agent, x: Value, gc: GcScope<'_, '_>) -> Self {
        // 1. If IsPromise(x) is true, then
        if let Value::Promise(promise) = x {
            // a. Let xConstructor be ? Get(x, "constructor").
            // b. If SameValue(xConstructor, C) is true, return x.
            // NOTE: Ignoring subclasses.
            promise
        } else {
            // 2. Let promiseCapability be ? NewPromiseCapability(C).
            let promise_capability = PromiseCapability::new(agent);
            // 3. Perform ? Call(promiseCapability.[[Resolve]], undefined, « x »).
            promise_capability.resolve(agent, x, gc);
            // 4. Return promiseCapability.[[Promise]].
            promise_capability.promise()
        }
    }
}

impl From<Promise> for PromiseIndex {
    fn from(val: Promise) -> Self {
        val.0
    }
}

impl From<PromiseIndex> for Promise {
    fn from(value: PromiseIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Promise {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Promise {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Promise> for Value {
    fn from(val: Promise) -> Self {
        Value::Promise(val)
    }
}

impl From<Promise> for Object {
    fn from(val: Promise) -> Self {
        Object::Promise(val)
    }
}

impl InternalSlots for Promise {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Promise;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl InternalMethods for Promise {}

impl CreateHeapData<PromiseHeapData, Promise> for Heap {
    fn create(&mut self, data: PromiseHeapData) -> Promise {
        self.promises.push(Some(data));
        Promise(PromiseIndex::last(&self.promises))
    }
}

impl Index<Promise> for Agent {
    type Output = PromiseHeapData;

    fn index(&self, index: Promise) -> &Self::Output {
        &self.heap.promises[index]
    }
}

impl IndexMut<Promise> for Agent {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        &mut self.heap.promises[index]
    }
}

impl Index<Promise> for Vec<Option<PromiseHeapData>> {
    type Output = PromiseHeapData;

    fn index(&self, index: Promise) -> &Self::Output {
        self.get(index.get_index())
            .expect("Promise out of bounds")
            .as_ref()
            .expect("Promise slot empty")
    }
}

impl IndexMut<Promise> for Vec<Option<PromiseHeapData>> {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Promise out of bounds")
            .as_mut()
            .expect("Promise slot empty")
    }
}

impl HeapMarkAndSweep for Promise {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promises.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.promises.shift_index(&mut self.0)
    }
}
