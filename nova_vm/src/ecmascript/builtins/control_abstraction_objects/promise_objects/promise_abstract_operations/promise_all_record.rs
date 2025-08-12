// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

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
pub struct PromiseAllRecordHeapData<'a> {
    pub remaining_unresolved_promise_count: u32,
    pub result_array: Array<'a>,
    pub promise: Promise<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseAllRecord<'a>(pub(crate) BaseIndex<'a, PromiseAllRecordHeapData<'a>>);

impl<'a> PromiseAllRecordHeapData<'a> {
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
            capability.resolve(
                agent,
                self.result_array.unbind().into_value(),
                gc.reborrow(),
            );
        }
    }
}

impl PromiseAllRecord<'_> {
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl Index<PromiseAllRecord<'_>> for Agent {
    type Output = PromiseAllRecordHeapData<'static>;

    fn index(&self, index: PromiseAllRecord) -> &Self::Output {
        &self.heap.promise_all_records[index]
    }
}

impl IndexMut<PromiseAllRecord<'_>> for Agent {
    fn index_mut(&mut self, index: PromiseAllRecord) -> &mut Self::Output {
        &mut self.heap.promise_all_records[index]
    }
}

impl Index<PromiseAllRecord<'_>> for Vec<Option<PromiseAllRecordHeapData<'static>>> {
    type Output = PromiseAllRecordHeapData<'static>;

    fn index(&self, index: PromiseAllRecord) -> &Self::Output {
        self.get(index.get_index())
            .expect("PromiseAllRecord out of bounds")
            .as_ref()
            .expect("PromiseAllRecord slot empty")
    }
}

impl IndexMut<PromiseAllRecord<'_>> for Vec<Option<PromiseAllRecordHeapData<'static>>> {
    fn index_mut(&mut self, index: PromiseAllRecord) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PromiseAllRecord out of bounds")
            .as_mut()
            .expect("PromiseAllRecord slot empty")
    }
}

impl HeapMarkAndSweep for PromiseAllRecordHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.result_array.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.result_array.sweep_values(compactions);
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

impl HeapMarkAndSweep for PromiseAllRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_all_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.promise_all_records.shift_index(&mut self.0);
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

impl<'a> CreateHeapData<PromiseAllRecordHeapData<'a>, PromiseAllRecord<'a>> for Heap {
    fn create(&mut self, data: PromiseAllRecordHeapData<'a>) -> PromiseAllRecord<'a> {
        self.promise_all_records.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<PromiseAllRecordHeapData<'static>>>();
        PromiseAllRecord(BaseIndex::last(&self.promise_all_records))
    }
}
