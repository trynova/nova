use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::define_property_or_throw,
        builtins::{
            Array,
            error::ErrorHeapData,
            promise::Promise,
            promise_objects::promise_abstract_operations::{
                promise_capability_records::PromiseCapability,
                promise_reaction_records::PromiseReactionType,
            },
        },
        execution::{Agent, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, IntoValue, OrdinaryObject, PropertyDescriptor, Value},
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
pub enum PromiseGroupType {
    All,
    AllSettled,
    Any,
}

#[derive(Debug, Clone, Copy)]
pub struct PromiseGroupRecord<'a> {
    pub(crate) promise_group_type: PromiseGroupType,
    pub(crate) remaining_elements_count: u32,
    pub(crate) result_array: Array<'a>,
    pub(crate) promise: Promise<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseGroup<'a>(BaseIndex<'a, PromiseGroupRecord<'static>>);

impl<'a> PromiseGroupRecord<'static> {
    fn take_result_and_promise(&mut self) -> (Array<'a>, Option<Promise<'a>>) {
        self.remaining_elements_count = self.remaining_elements_count.saturating_sub(1);

        if self.remaining_elements_count > 0 {
            (self.result_array, None)
        } else {
            (self.result_array, Some(self.promise))
        }
    }
}

impl<'a> PromiseGroup<'a> {
    pub(crate) fn settle(
        self,
        agent: &mut Agent,
        reaction_type: PromiseReactionType,
        index: u32,
        value: Value<'a>,
        mut gc: GcScope<'a, '_>,
    ) {
        let value = value.bind(gc.nogc());
        let record = self.get(agent);

        match record.promise_group_type {
            PromiseGroupType::All => match reaction_type {
                PromiseReactionType::Fulfill => {
                    self.fulfill(agent, index, value.unbind(), gc.reborrow());
                }
                PromiseReactionType::Reject => {
                    self.immediately_reject(agent, value.unbind(), gc.nogc());
                }
            },
            PromiseGroupType::AllSettled => {
                let obj = self
                    .to_all_settled_obj(agent, reaction_type, value.unbind(), gc.nogc())
                    .bind(gc.nogc());
                self.fulfill(agent, index, obj.unbind(), gc.reborrow());
            }
            PromiseGroupType::Any => match reaction_type {
                PromiseReactionType::Fulfill => {
                    self.immediately_resolve(agent, value.unbind(), gc.reborrow());
                }
                PromiseReactionType::Reject => {
                    self.reject(agent, index, value.unbind(), gc.reborrow());
                }
            },
        }
    }

    pub(crate) fn fulfill(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        mut gc: GcScope<'a, '_>,
    ) {
        let promise_group = self.bind(gc.nogc());
        let value = value.bind(gc.nogc());

        let promise_group_record = promise_group.get_mut(agent);
        let (result_array, promise_to_resolve) = promise_group_record.take_result_and_promise();

        let elements = result_array.as_mut_slice(agent);
        elements[index as usize] = Some(value.unbind());

        if let Some(promise_to_resolve) = promise_to_resolve {
            let capability = PromiseCapability::from_promise(promise_to_resolve, true);
            capability.resolve(agent, result_array.into_value().unbind(), gc.reborrow());
        }
    }

    pub(crate) fn reject(
        self,
        agent: &mut Agent,
        index: u32,
        error: Value<'a>,
        mut gc: GcScope<'a, '_>,
    ) {
        let promise_group = self.bind(gc.nogc());
        let error = error.bind(gc.nogc());

        let promise_group_record = promise_group.get_mut(agent);
        let (result_array, promise_to_resolve) = promise_group_record.take_result_and_promise();

        let elements = result_array.as_mut_slice(agent);
        elements[index as usize] = Some(error.unbind());

        if let Some(promise_to_resolve) = promise_to_resolve {
            let aggregate_error = agent.heap.create(ErrorHeapData::new(
                ExceptionType::AggregateError,
                None,
                None,
            ));

            let capability = PromiseCapability::from_promise(promise_to_resolve, true);
            if let Err(err) = define_property_or_throw(
                agent,
                aggregate_error,
                BUILTIN_STRING_MEMORY.errors.into(),
                PropertyDescriptor {
                    value: Some(result_array.into_value().unbind()),
                    writable: Some(true),
                    get: None,
                    set: None,
                    enumerable: Some(true),
                    configurable: Some(true),
                },
                gc.reborrow(),
            ) {
                capability.reject(agent, err.value().unbind(), gc.nogc());
            } else {
                capability.reject(agent, aggregate_error.into_value(), gc.nogc());
            }
        }
    }

    pub(crate) fn immediately_resolve(
        self,
        agent: &mut Agent,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    ) {
        let value = value.bind(gc.nogc());
        let promise_group = self.bind(gc.nogc());
        let data = promise_group.get_mut(agent);

        let capability = PromiseCapability::from_promise(data.promise, true);
        capability.resolve(agent, value.unbind(), gc);
    }

    pub(crate) fn immediately_reject(
        self,
        agent: &mut Agent,
        value: Value<'a>,
        gc: NoGcScope<'a, '_>,
    ) {
        let value = value.bind(gc);
        let promise_group = self.bind(gc);
        let data = promise_group.get_mut(agent);

        let capability = PromiseCapability::from_promise(data.promise, true);
        capability.reject(agent, value.unbind(), gc);
    }

    fn to_all_settled_obj(
        self,
        agent: &mut Agent,
        reaction_type: PromiseReactionType,
        value: Value<'a>,
        gc: NoGcScope<'a, '_>,
    ) -> Value<'a> {
        let value = value.bind(gc);

        let entries = vec![
            ObjectEntry::new_data_entry(
                BUILTIN_STRING_MEMORY.status.into(),
                match reaction_type {
                    PromiseReactionType::Fulfill => BUILTIN_STRING_MEMORY.fulfilled.into(),
                    PromiseReactionType::Reject => BUILTIN_STRING_MEMORY.rejected.into(),
                },
            ),
            ObjectEntry::new_data_entry(
                match reaction_type {
                    PromiseReactionType::Fulfill => BUILTIN_STRING_MEMORY.value.into(),
                    PromiseReactionType::Reject => BUILTIN_STRING_MEMORY.reason.into(),
                },
                value.unbind(),
            ),
        ];

        let obj = OrdinaryObject::create_object(
            agent,
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into(),
            ),
            &entries,
        )
        .bind(gc);

        obj.into_value().unbind()
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

    pub(crate) const _DEF: Self = { Self(BaseIndex::from_u32_index(0)) };
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
        let Self {
            promise_group_type: _,
            remaining_elements_count: _,
            result_array,
            promise,
        } = self;
        result_array.mark_values(queues);
        promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            promise_group_type: _,
            remaining_elements_count: _,
            result_array,
            promise,
        } = self;
        result_array.sweep_values(compactions);
        promise.sweep_values(compactions);
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
        PromiseGroup(BaseIndex::last(&self.promise_group_records))
    }
}
