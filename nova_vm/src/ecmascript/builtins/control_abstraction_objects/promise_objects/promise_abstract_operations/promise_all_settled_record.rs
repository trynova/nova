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
        types::{BUILTIN_STRING_MEMORY, IntoValue, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, ObjectEntry, WorkQueues,
        indexes::BaseIndex,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PromiseAllSettledRecord<'a> {
    pub(crate) remaining_elements_count: u32,
    pub(crate) result_array: Array<'a>,
    pub(crate) promise: Promise<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseAllSettled<'a>(BaseIndex<'a, PromiseAllSettledRecord<'static>>);

impl<'a> PromiseAllSettled<'a> {
    pub(crate) fn on_promise_fulfilled(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    ) {
        let promise_all = self.bind(gc.nogc());
        let value = value.bind(gc.nogc());

        let result_array = promise_all.get_result_array(agent, gc.nogc());

        // Let obj be OrdinaryObjectCreate(%Object.prototype%).
        // 10. Perform ! CreateDataPropertyOrThrow(obj, "status", "fulfilled").
        // 11. Perform ! CreateDataPropertyOrThrow(obj, "value", x).
        let obj = OrdinaryObject::create_object(
            agent,
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into(),
            ),
            &[
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.status.into(),
                    BUILTIN_STRING_MEMORY.fulfilled.into(),
                ),
                ObjectEntry::new_data_entry(BUILTIN_STRING_MEMORY.value.into(), value.unbind()),
            ],
        )
        .bind(gc.nogc());

        let elements = result_array.as_mut_slice(agent);
        elements[index as usize] = Some(obj.unbind().into_value());

        let data = promise_all.get_mut(agent);

        // 13. Set remainingElementsCount.[[Value]] to remainingElementsCount.[[Value]] - 1.
        data.remaining_elements_count = data.remaining_elements_count.saturating_sub(1);

        // 14. If remainingElementsCount.[[Value]] = 0, then
        if data.remaining_elements_count == 0 {
            // a. Let valuesArray be CreateArrayFromList(values).
            // b. Return ? Call(promiseCapability.[[Resolve]], undefined, « valuesArray »).
            let capability = PromiseCapability::from_promise(data.promise, true);
            capability.resolve(agent, result_array.into_value().unbind(), gc);
        }
    }

    pub(crate) fn on_promise_rejected(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    ) {
        let promise_all = self.bind(gc.nogc());
        let value = value.bind(gc.nogc());

        let result_array = promise_all.get_result_array(agent, gc.nogc());

        // Let obj be OrdinaryObjectCreate(%Object.prototype%).
        // 10. Perform ! CreateDataPropertyOrThrow(obj, "status", "rejected").
        // 11. Perform ! CreateDataPropertyOrThrow(obj, "reason", x).
        let obj = OrdinaryObject::create_object(
            agent,
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into(),
            ),
            &[
                ObjectEntry::new_data_entry(
                    BUILTIN_STRING_MEMORY.status.into(),
                    BUILTIN_STRING_MEMORY.rejected.into(),
                ),
                ObjectEntry::new_data_entry(BUILTIN_STRING_MEMORY.reason.into(), value.unbind()),
            ],
        )
        .bind(gc.nogc());

        let elements = result_array.as_mut_slice(agent);
        elements[index as usize] = Some(obj.unbind().into_value());

        let data = promise_all.get_mut(agent);

        // 13. Set remainingElementsCount.[[Value]] to remainingElementsCount.[[Value]] - 1.
        data.remaining_elements_count = data.remaining_elements_count.saturating_sub(1);

        // 14. If remainingElementsCount.[[Value]] = 0, then
        if data.remaining_elements_count == 0 {
            // a. Let valuesArray be CreateArrayFromList(values).
            // b. Return ? Call(promiseCapability.[[Resolve]], undefined, « valuesArray »).
            let capability = PromiseCapability::from_promise(data.promise, true);
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

    pub fn get(self, agent: &Agent) -> &PromiseAllSettledRecord<'a> {
        agent
            .heap
            .promise_all_settled_records
            .get(self.get_index())
            .expect("PromiseAllSettledRecord not found")
    }

    pub fn get_mut(self, agent: &mut Agent) -> &mut PromiseAllSettledRecord<'static> {
        agent
            .heap
            .promise_all_settled_records
            .get_mut(self.get_index())
            .expect("PromiseAllSettledRecord not found")
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }
}

impl AsRef<[PromiseAllSettledRecord<'static>]> for Agent {
    fn as_ref(&self) -> &[PromiseAllSettledRecord<'static>] {
        &self.heap.promise_all_settled_records
    }
}

impl AsMut<[PromiseAllSettledRecord<'static>]> for Agent {
    fn as_mut(&mut self) -> &mut [PromiseAllSettledRecord<'static>] {
        &mut self.heap.promise_all_settled_records
    }
}

impl HeapMarkAndSweep for PromiseAllSettledRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            remaining_elements_count: _,
            result_array,
            promise,
        } = self;
        result_array.mark_values(queues);
        promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            remaining_elements_count: _,
            result_array,
            promise,
        } = self;
        result_array.sweep_values(compactions);
        promise.sweep_values(compactions);
    }
}

impl Rootable for PromiseAllSettled<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::PromiseAllSettled(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::PromiseAllSettled(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for PromiseAllSettled<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_all_settled_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .promise_all_settled_records
            .shift_index(&mut self.0);
    }
}

bindable_handle!(PromiseAllSettledRecord);
bindable_handle!(PromiseAllSettled);

impl<'a> CreateHeapData<PromiseAllSettledRecord<'a>, PromiseAllSettled<'a>> for Heap {
    fn create(&mut self, data: PromiseAllSettledRecord<'a>) -> PromiseAllSettled<'a> {
        self.promise_all_settled_records.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PromiseAllSettledRecord<'static>>();
        PromiseAllSettled(BaseIndex::last_t(&self.promise_all_settled_records))
    }
}
