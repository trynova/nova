// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable};
use crate::engine::Scoped;
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
pub struct Promise<'a>(pub(crate) PromiseIndex<'a>);

impl<'a> Promise<'a> {
    /// Unbind this Promise from its current lifetime. This is necessary to use
    /// the Promise as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> Promise<'static> {
        unsafe { std::mem::transmute::<Self, Promise<'static>>(self) }
    }

    // Bind this Promise to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your Promises cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let promise = promise.bind(&gc);
    // ```
    // to make sure that the unbound Promise cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> Promise<'gc> {
        unsafe { std::mem::transmute::<Promise, Promise<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Promise<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// [27.2.4.7.1 PromiseResolve ( C, x )](https://tc39.es/ecma262/#sec-promise-resolve)
    pub fn resolve(agent: &mut Agent, x: Value, mut gc: GcScope<'a, '_>) -> Self {
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
            promise_capability.resolve(agent, x, gc.reborrow());
            // 4. Return promiseCapability.[[Promise]].
            promise_capability.promise().bind(gc.into_nogc())
        }
    }
}

impl IntoValue for Promise<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for Promise<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl From<Promise<'_>> for Value {
    fn from(val: Promise) -> Self {
        Value::Promise(val.unbind())
    }
}

impl<'a> From<Promise<'a>> for Object<'a> {
    fn from(val: Promise) -> Self {
        Object::Promise(val.unbind())
    }
}

impl<'a> InternalSlots<'a> for Promise<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Promise;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }
}

impl<'a> InternalMethods<'a> for Promise<'a> {}

impl CreateHeapData<PromiseHeapData, Promise<'static>> for Heap {
    fn create(&mut self, data: PromiseHeapData) -> Promise<'static> {
        self.promises.push(Some(data));
        Promise(PromiseIndex::last(&self.promises))
    }
}

impl Index<Promise<'_>> for Agent {
    type Output = PromiseHeapData;

    fn index(&self, index: Promise) -> &Self::Output {
        &self.heap.promises[index]
    }
}

impl IndexMut<Promise<'_>> for Agent {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        &mut self.heap.promises[index]
    }
}

impl Index<Promise<'_>> for Vec<Option<PromiseHeapData>> {
    type Output = PromiseHeapData;

    fn index(&self, index: Promise) -> &Self::Output {
        self.get(index.get_index())
            .expect("Promise out of bounds")
            .as_ref()
            .expect("Promise slot empty")
    }
}

impl IndexMut<Promise<'_>> for Vec<Option<PromiseHeapData>> {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Promise out of bounds")
            .as_mut()
            .expect("Promise slot empty")
    }
}

impl Rootable for Promise<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Promise(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Promise(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for Promise<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promises.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.promises.shift_index(&mut self.0)
    }
}
