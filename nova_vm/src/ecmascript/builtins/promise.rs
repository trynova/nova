// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use data::PromiseState;

use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable, Scopable};
use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        CreateHeapData, Heap, HeapMarkAndSweep,
        indexes::{BaseIndex, PromiseIndex},
    },
};

use self::data::PromiseHeapData;

use super::control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Promise<'a>(pub(crate) PromiseIndex<'a>);

impl<'a> Promise<'a> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn set_already_resolved(self, agent: &mut Agent) {
        match &mut agent[self].promise_state {
            PromiseState::Pending { is_resolved, .. } => *is_resolved = true,
            _ => unreachable!(),
        };
    }

    /// [27.2.4.7.1 PromiseResolve ( C, x )](https://tc39.es/ecma262/#sec-promise-resolve)
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Promise<'_> {
    type Of<'a> = Promise<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> IntoValue<'a> for Promise<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for Promise<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<Promise<'a>> for Value<'a> {
    fn from(value: Promise<'a>) -> Self {
        Value::Promise(value)
    }
}

impl<'a> From<Promise<'a>> for Object<'a> {
    fn from(value: Promise<'a>) -> Self {
        Object::Promise(value)
    }
}

impl<'a> InternalSlots<'a> for Promise<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Promise;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for Promise<'a> {}

impl<'a> CreateHeapData<PromiseHeapData<'a>, Promise<'a>> for Heap {
    fn create(&mut self, data: PromiseHeapData<'a>) -> Promise<'a> {
        self.promises.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<PromiseHeapData<'static>>>();
        Promise(PromiseIndex::last(&self.promises))
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for PromiseHeapData<'_> {
    type Of<'a> = PromiseHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl Index<Promise<'_>> for Agent {
    type Output = PromiseHeapData<'static>;

    fn index(&self, index: Promise) -> &Self::Output {
        &self.heap.promises[index]
    }
}

impl IndexMut<Promise<'_>> for Agent {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        &mut self.heap.promises[index]
    }
}

impl Index<Promise<'_>> for Vec<Option<PromiseHeapData<'static>>> {
    type Output = PromiseHeapData<'static>;

    fn index(&self, index: Promise) -> &Self::Output {
        self.get(index.get_index())
            .expect("Promise out of bounds")
            .as_ref()
            .expect("Promise slot empty")
    }
}

impl IndexMut<Promise<'_>> for Vec<Option<PromiseHeapData<'static>>> {
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
