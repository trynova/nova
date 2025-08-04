// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::{
            Array,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::Agent,
        types::Value,
    },
    engine::context::{Bindable, NoGcScope},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues, indexes::BaseIndex,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PromiseAllRecord<'a> {
    promise_capability: &'a PromiseCapability<'a>,
    remaining_unresolved_promise_count: u32,
    result_array: Array<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseAllRecordHeapData<'a>(BaseIndex<'a, PromiseAllRecord<'a>>);

impl<'a> PromiseAllRecord<'a> {
    pub(crate) fn new(
        agent: &mut Agent,
        promise_capability: &'a PromiseCapability<'a>,
        num_promises: u32,
        gc: NoGcScope<'a, '_>,
    ) -> Self {
        let undefined_values = vec![Value::Undefined; num_promises as usize];
        let result_array = Array::from_slice(agent, &undefined_values, gc);
        Self {
            promise_capability,
            remaining_unresolved_promise_count: num_promises,
            result_array,
        }
    }

    pub(crate) fn on_promise_fufilled(
        &mut self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        gc: NoGcScope<'a, '_>,
    ) {
        // Update the result array at the specified index by reconstructing it
        let array_slice = self.result_array.as_slice(agent);
        let new_values: Vec<Value> = array_slice
            .iter()
            .enumerate()
            .map(|(i, opt_val)| {
                if i == index as usize {
                    value // Use the new value at this index
                } else {
                    opt_val.unwrap_or(Value::Undefined) // Keep existing or default
                }
            })
            .collect();

        self.result_array = Array::from_slice(agent, &new_values, gc);

        self.remaining_unresolved_promise_count -= 1;
        if self.remaining_unresolved_promise_count == 0 {
            eprintln!(
                "All promises fulfilled, should resolve main promise: {:#?}",
                self.result_array
            );
            // self.promise_capability
            //     .resolve(agent, self.result_array, gc.into_nogc());
        }
    }
}

impl PromiseAllRecordHeapData<'_> {
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl Index<PromiseAllRecordHeapData<'_>> for Agent {
    type Output = PromiseAllRecord<'static>;

    fn index(&self, index: PromiseAllRecordHeapData) -> &Self::Output {
        &self.heap.promise_all_records[index]
    }
}

impl IndexMut<PromiseAllRecordHeapData<'_>> for Agent {
    fn index_mut(&mut self, index: PromiseAllRecordHeapData) -> &mut Self::Output {
        &mut self.heap.promise_all_records[index]
    }
}

impl Index<PromiseAllRecordHeapData<'_>> for Vec<Option<PromiseAllRecord<'static>>> {
    type Output = PromiseAllRecord<'static>;

    fn index(&self, index: PromiseAllRecordHeapData) -> &Self::Output {
        self.get(index.get_index())
            .expect("PromiseAllRecord out of bounds")
            .as_ref()
            .expect("PromiseAllRecord slot empty")
    }
}

impl IndexMut<PromiseAllRecordHeapData<'_>> for Vec<Option<PromiseAllRecord<'static>>> {
    fn index_mut(&mut self, index: PromiseAllRecordHeapData) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PromiseAllRecord out of bounds")
            .as_mut()
            .expect("PromiseAllRecord slot empty")
    }
}

impl HeapMarkAndSweep for PromiseAllRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.result_array.mark_values(queues);
        self.promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.result_array.sweep_values(compactions);
        self.promise_capability.sweep_values(compactions);
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

impl HeapMarkAndSweep for PromiseAllRecordHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_all_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.promise_all_records.shift_index(&mut self.0);
    }
}

unsafe impl Bindable for PromiseAllRecordHeapData<'_> {
    type Of<'a> = PromiseAllRecordHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> CreateHeapData<PromiseAllRecord<'a>, PromiseAllRecordHeapData<'a>> for Heap {
    fn create(&mut self, data: PromiseAllRecord<'a>) -> PromiseAllRecordHeapData<'a> {
        self.promise_all_records.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<PromiseAllRecord<'static>>>();
        PromiseAllRecordHeapData(BaseIndex::last(&self.promise_all_records))
    }
}
