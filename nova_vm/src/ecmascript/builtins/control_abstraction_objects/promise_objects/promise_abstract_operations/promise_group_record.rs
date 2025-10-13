use crate::{
    ecmascript::{
        builtins::promise_objects::promise_abstract_operations::{
            promise_all_record::PromiseAllRecord,
            promise_all_settled_record::PromiseAllSettledRecord,
        },
        execution::Agent,
        types::Value,
    },
    engine::{
        context::{Bindable, GcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues, indexes::BaseIndex,
    },
};

#[derive(Debug, Clone, Copy)]
pub enum PromiseGroupRecord<'a> {
    PromiseAll(PromiseAllRecord<'a>),
    PromiseAllSettled(PromiseAllSettledRecord<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseGroup<'a>(BaseIndex<'a, PromiseGroupRecord<'static>>);

impl<'a> PromiseGroup<'a> {
    pub(crate) fn on_promise_fulfilled(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        mut gc: GcScope<'a, '_>,
    ) {
        let value = value.bind(gc.nogc());
        let promise_group = self.bind(gc.nogc());

        let promise_group = promise_group.get_mut(agent);
        match promise_group {
            PromiseGroupRecord::PromiseAll(promise_all) => {
                // i. Set remainingElementsCount.[[Value]] to remainingElementsCount.[[Value]] - 1.
                promise_all.remaining_elements_count =
                    promise_all.remaining_elements_count.saturating_sub(1);
                promise_all.on_promise_fulfilled(agent, index, value.unbind(), gc.reborrow());
            }
            PromiseGroupRecord::PromiseAllSettled(promise_all_settled) => {
                // promise_all_settled.on_promise_fulfilled(agent, index, value, gc)
                todo!()
            }
        };
    }

    pub(crate) fn on_promise_rejected(
        self,
        agent: &mut Agent,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    ) {
        let value = value.bind(gc.nogc());

        let promise_group_record = self.get(agent);
        match promise_group_record {
            PromiseGroupRecord::PromiseAll(promise_all) => {
                promise_all.on_promise_rejected(agent, value.unbind(), gc.nogc())
            }
            PromiseGroupRecord::PromiseAllSettled(promise_all_settled) => {
                todo!()
            }
        }
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn get(self, agent: &Agent) -> &PromiseGroupRecord<'a> {
        agent
            .heap
            .promise_group_records
            .get(self.get_index())
            .expect("PromiseGroupRecord not found")
    }

    pub fn get_mut(self, agent: &mut Agent) -> &mut PromiseGroupRecord<'static> {
        agent
            .heap
            .promise_group_records
            .get_mut(self.get_index())
            .expect("PromiseGroupRecord not found")
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }
}

impl AsRef<[PromiseGroupRecord<'static>]> for Agent {
    fn as_ref(&self) -> &[PromiseGroupRecord<'static>] {
        &self.heap.promise_group_records
    }
}

impl AsMut<[PromiseGroupRecord<'static>]> for Agent {
    fn as_mut(&mut self) -> &mut [PromiseGroupRecord<'static>] {
        &mut self.heap.promise_group_records
    }
}

impl HeapMarkAndSweep for PromiseGroupRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            PromiseGroupRecord::PromiseAll(promise_all) => promise_all.mark_values(queues),
            PromiseGroupRecord::PromiseAllSettled(promise_all_settled) => {
                promise_all_settled.mark_values(queues)
            }
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            PromiseGroupRecord::PromiseAll(promise_all) => promise_all.sweep_values(compactions),
            PromiseGroupRecord::PromiseAllSettled(promise_all_settled) => {
                promise_all_settled.sweep_values(compactions)
            }
        }
    }
}

impl Rootable for PromiseGroup<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::PromiseGroup(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::PromiseGroup(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for PromiseGroup<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_group_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.promise_group_records.shift_index(&mut self.0);
    }
}

bindable_handle!(PromiseGroupRecord);
bindable_handle!(PromiseGroup);

impl<'a> CreateHeapData<PromiseGroupRecord<'a>, PromiseGroup<'a>> for Heap {
    fn create(&mut self, data: PromiseGroupRecord<'a>) -> PromiseGroup<'a> {
        self.promise_group_records.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PromiseGroupRecord<'static>>();
        PromiseGroup(BaseIndex::last_t(&self.promise_group_records))
    }
}
