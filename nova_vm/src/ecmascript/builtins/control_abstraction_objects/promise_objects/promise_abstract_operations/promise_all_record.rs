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
    engine::context::{Bindable, GcScope, NoGcScope, bindable_handle},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues, indexes::BaseIndex,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PromiseAllRecord<'a> {
    pub(crate) remaining_unresolved_promise_count: u32,
    pub(crate) result_array: Array<'a>,
    pub(crate) promise: Promise<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseAll<'a>(BaseIndex<'a, PromiseAllRecord<'static>>);

impl<'a> PromiseAll<'a> {
    pub(crate) fn on_promise_fulfilled(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    ) {
        let promise_all = self.bind(gc.nogc());
        let value = value.bind(gc.nogc());

        let result_array = self.get_result_array(agent, gc.nogc());

        let elements = result_array.as_mut_slice(agent);
        elements[index as usize] = Some(value.unbind());

        let data = promise_all.get_mut(agent);
        data.remaining_unresolved_promise_count =
            data.remaining_unresolved_promise_count.saturating_sub(1);
        if data.remaining_unresolved_promise_count == 0 {
            let capability = PromiseCapability::from_promise(data.promise, false);
            capability.resolve(agent, result_array.into_value().unbind(), gc);
        }
    }

    pub(crate) fn get_result_array(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Array<'a> {
        let data = self.get(agent);
        data.result_array.bind(gc).unbind()
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    fn get(self, agent: &Agent) -> &PromiseAllRecord<'a> {
        agent
            .heap
            .promise_all_records
            .get(self.get_index())
            .expect("PromiseAllRecord not found")
    }

    fn get_mut(self, agent: &mut Agent) -> &mut PromiseAllRecord<'static> {
        agent
            .heap
            .promise_all_records
            .get_mut(self.get_index())
            .expect("PromiseAllRecord not found")
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
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

impl HeapMarkAndSweep for PromiseAll<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_all_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.promise_all_records.shift_index(&mut self.0);
    }
}

bindable_handle!(PromiseAllRecord);
bindable_handle!(PromiseAll);

impl<'a> CreateHeapData<PromiseAllRecord<'a>, PromiseAll<'a>> for Heap {
    fn create(&mut self, data: PromiseAllRecord<'a>) -> PromiseAll<'a> {
        self.promise_all_records.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PromiseAllRecord<'static>>();
        PromiseAll(BaseIndex::last_t(&self.promise_all_records))
    }
}
