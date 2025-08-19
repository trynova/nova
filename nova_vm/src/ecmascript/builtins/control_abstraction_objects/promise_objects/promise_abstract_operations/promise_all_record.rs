// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            Array, promise::Promise,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::Agent,
        types::{IntoValue, Value},
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues, indexes::BaseIndex,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PromiseAllRecord<'a> {
    pub remaining_unresolved_promise_count: u32,
    pub result_array: Array<'a>,
    pub promise: Promise<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseAll<'a>(pub(crate) BaseIndex<'a, PromiseAllRecord<'a>>);

impl<'a> PromiseAllRecord<'a> {
    pub(crate) fn on_promise_fufilled(
        &mut self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        mut gc: GcScope<'a, '_>,
    ) {
        value.bind(gc.nogc());
        let elements = self.result_array.as_mut_slice(agent);
        elements[index as usize] = Some(value.unbind());

        self.remaining_unresolved_promise_count -= 1;
        if self.remaining_unresolved_promise_count == 0 {
            eprintln!("Promise fulfilled: {:#?}", elements);
            let capability = PromiseCapability::from_promise(self.promise.unbind(), true);
            capability.resolve(agent, self.result_array.into_value().unbind(), gc);
        }
    }
}

impl PromiseAll<'_> {
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl AsRef<[PromiseAllRecord<'static>]> for Agent {
    fn as_ref(&self) -> &[PromiseAllRecord<'static>] {
        &self.heap.promise_all_records
    }
}

impl AsMut<[PromiseAllRecord<'static>]> for Agent {
    fn as_mut(&mut self) -> &mut [PromiseAllRecord<'static>] {
        &mut self.heap.promise_all_records
    }
}

impl HeapMarkAndSweep for PromiseAllRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            remaining_unresolved_promise_count: _,
            result_array,
            promise,
        } = self;
        result_array.mark_values(queues);
        promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            remaining_unresolved_promise_count: _,
            result_array,
            promise,
        } = self;
        result_array.sweep_values(compactions);
        promise.sweep_values(compactions);
    }
}

unsafe impl Bindable for PromiseAllRecord<'_> {
    type Of<'a> = PromiseAllRecord<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for PromiseAll<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_all_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.promise_all_records.shift_index(&mut self.0);
    }
}

unsafe impl Bindable for PromiseAll<'_> {
    type Of<'a> = PromiseAll<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> CreateHeapData<PromiseAllRecord<'a>, PromiseAll<'a>> for Heap {
    fn create(&mut self, data: PromiseAllRecord<'a>) -> PromiseAll<'a> {
        self.promise_all_records.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PromiseAllRecord<'static>>();
        PromiseAll(BaseIndex::last_t(&self.promise_all_records))
    }
}
