// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData, Value,
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
pub struct Promise<'gen>(pub(crate) PromiseIndex<'gen>);

impl<'gen> Promise<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// [27.2.4.7.1 PromiseResolve ( C, x )](https://tc39.es/ecma262/#sec-promise-resolve)
    pub fn resolve(agent: &mut Agent, x: Value) -> Self {
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
            promise_capability.resolve(agent, x);
            // 4. Return promiseCapability.[[Promise]].
            promise_capability.promise()
        }
    }
}

impl<'gen> From<Promise<'gen>> for PromiseIndex<'gen> {
    fn from(val: Promise<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<PromiseIndex<'gen>> for Promise<'gen> {
    fn from(value: PromiseIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for Promise<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for Promise<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<Promise<'gen>> for Value<'gen> {
    fn from(val: Promise<'gen>) -> Self {
        Value::Promise(val)
    }
}

impl<'gen> From<Promise<'gen>> for Object<'gen> {
    fn from(val: Promise<'gen>) -> Self {
        Object::Promise(val)
    }
}

impl<'gen> InternalSlots<'gen> for Promise<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Promise;

    #[inline(always)]
    fn get_backing_object<'b>(self, agent: &'b Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> where 'gen: 'b {
        agent[self].object_index
    }

    fn create_backing_object<'b>(self, agent: &'b mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> where 'gen: 'b {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl<'gen> InternalMethods<'gen> for Promise<'gen> {}

impl<'gen> CreateHeapData<PromiseHeapData<'gen>, Promise<'gen>> for Heap<'gen> {
    fn create(&mut self, data: PromiseHeapData<'gen>) -> Promise<'gen> {
        self.promises.push(Some(data));
        Promise(PromiseIndex::last(&self.promises))
    }
}

impl<'gen> Index<Promise<'gen>> for Agent<'gen> {
    type Output = PromiseHeapData<'gen>;

    fn index(&self, index: Promise<'gen>) -> &Self::Output {
        &self.heap.promises[index]
    }
}

impl<'gen> IndexMut<Promise<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: Promise<'gen>) -> &mut Self::Output {
        &mut self.heap.promises[index]
    }
}

impl<'gen> Index<Promise<'gen>> for Vec<Option<PromiseHeapData<'gen>>> {
    type Output = PromiseHeapData<'gen>;

    fn index(&self, index: Promise<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Promise out of bounds")
            .as_ref()
            .expect("Promise slot empty")
    }
}

impl<'gen> IndexMut<Promise<'gen>> for Vec<Option<PromiseHeapData<'gen>>> {
    fn index_mut(&mut self, index: Promise<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Promise out of bounds")
            .as_mut()
            .expect("Promise slot empty")
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for Promise<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.promises.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.promises.shift_index(&mut self.0)
    }
}
