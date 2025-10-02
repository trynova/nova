// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::{
            Array, async_generator_objects::AsyncGenerator,
            control_abstraction_objects::async_function_objects::await_reaction::AwaitReaction,
            promise::Promise,
        },
        execution::Agent,
        scripts_and_modules::module::module_semantics::{
            abstract_module_records::AbstractModule, source_text_module_records::SourceTextModule,
        },
        types::{Function, Object},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues, indexes::BaseIndex,
    },
};

use super::promise_capability_records::PromiseCapability;

/// \[\[Type\]\]
///
/// fulfill or reject
///
/// The \[\[Type\]\] is used when \[\[Handler\]\] is empty to allow for
/// behaviour specific to the settlement type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromiseReactionType {
    Fulfill,
    Reject,
}

/// \[\[Handler\]\]
///
/// a JobCallback Record or empty
///
/// The function that should be applied to the incoming value, and whose
/// return value will govern what happens to the derived promise. If
/// \[\[Handler\]\] is empty, a function that depends on the value of
/// \[\[Type\]\] will be used instead.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PromiseReactionHandler<'a> {
    JobCallback(Function<'a>),
    Await(AwaitReaction<'a>),
    AsyncGenerator(AsyncGenerator<'a>),
    /// See `unwrap` Abstract Closure in step 9 of
    /// AsyncFromSyncIteratorContinuation.
    AsyncFromSyncIterator {
        done: bool,
    },
    AsyncFromSyncIteratorClose(Object<'a>),
    AsyncModule(SourceTextModule<'a>),
    DynamicImport {
        promise: Promise<'a>,
        module: AbstractModule<'a>,
    },
    DynamicImportEvaluate {
        promise: Promise<'a>,
        module: AbstractModule<'a>,
    },
    PromiseAll {
        result_promise: Promise<'a>,
        remaining_unresolved_promise_count: usize,
        result_array: Array<'a>,
    },
    Empty,
}

impl HeapMarkAndSweep for PromiseReactionHandler<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Self::JobCallback(function) => function.mark_values(queues),
            Self::Await(await_reaction_identifier) => await_reaction_identifier.mark_values(queues),
            Self::AsyncGenerator(async_generator) => async_generator.mark_values(queues),
            Self::AsyncFromSyncIterator { done: _ } => {}
            Self::AsyncFromSyncIteratorClose(object) => object.mark_values(queues),
            Self::AsyncModule(module) => module.mark_values(queues),
            Self::DynamicImport { promise, module }
            | Self::DynamicImportEvaluate { promise, module } => {
                promise.mark_values(queues);
                module.mark_values(queues);
            }
            Self::PromiseAll {
                result_promise: promise,
                remaining_unresolved_promise_count: _,
                result_array,
            } => {
                promise.mark_values(queues);
                result_array.mark_values(queues);
            }
            Self::Empty => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::JobCallback(function) => function.sweep_values(compactions),
            Self::Await(await_reaction_identifier) => {
                await_reaction_identifier.sweep_values(compactions)
            }
            Self::AsyncGenerator(async_generator) => async_generator.sweep_values(compactions),
            Self::AsyncFromSyncIterator { done: _ } => {}
            Self::AsyncFromSyncIteratorClose(object) => object.sweep_values(compactions),
            Self::AsyncModule(module) => module.sweep_values(compactions),
            Self::DynamicImport { promise, module }
            | Self::DynamicImportEvaluate { promise, module } => {
                promise.sweep_values(compactions);
                module.sweep_values(compactions);
            }
            Self::PromiseAll {
                result_promise: promise,
                remaining_unresolved_promise_count: _,
                result_array,
            } => {
                promise.sweep_values(compactions);
                result_array.sweep_values(compactions);
            }
            Self::Empty => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromiseReactionRecord<'a> {
    /// \[\[Capability\]\]
    ///
    /// a PromiseCapability Record or undefined
    ///
    /// The capabilities of the promise for which this record provides a
    /// reaction handler.
    pub(crate) capability: Option<PromiseCapability<'a>>,
    /// \[\[Type\]\]
    pub(crate) reaction_type: PromiseReactionType,
    /// \[\[Handler\]\]
    ///
    /// a JobCallback Record or empty
    ///
    /// The function that should be applied to the incoming value, and whose
    /// return value will govern what happens to the derived promise. If
    /// \[\[Handler\]\] is empty, a function that depends on the value of
    /// \[\[Type\]\] will be used instead.
    pub(crate) handler: PromiseReactionHandler<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseReaction<'a>(BaseIndex<'a, PromiseReactionRecord<'static>>);

impl PromiseReaction<'_> {
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl Index<PromiseReaction<'_>> for Agent {
    type Output = PromiseReactionRecord<'static>;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        &self.heap.promise_reaction_records[index]
    }
}

impl IndexMut<PromiseReaction<'_>> for Agent {
    fn index_mut(&mut self, index: PromiseReaction) -> &mut Self::Output {
        &mut self.heap.promise_reaction_records[index]
    }
}

impl Index<PromiseReaction<'_>> for Vec<Option<PromiseReactionRecord<'static>>> {
    type Output = PromiseReactionRecord<'static>;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        self.get(index.get_index())
            .expect("PromiseReaction out of bounds")
            .as_ref()
            .expect("PromiseReaction slot empty")
    }
}

impl IndexMut<PromiseReaction<'_>> for Vec<Option<PromiseReactionRecord<'static>>> {
    fn index_mut(&mut self, index: PromiseReaction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PromiseReaction out of bounds")
            .as_mut()
            .expect("PromiseReaction slot empty")
    }
}

bindable_handle!(PromiseReaction);

impl HeapMarkAndSweep for PromiseReaction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_reaction_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .promise_reaction_records
            .shift_index(&mut self.0);
    }
}

impl Rootable for PromiseReaction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::PromiseReaction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        if let HeapRootData::PromiseReaction(data) = heap_data {
            Some(data)
        } else {
            None
        }
    }
}

bindable_handle!(PromiseReactionRecord);

impl HeapMarkAndSweep for PromiseReactionRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            capability,
            reaction_type: _,
            handler,
        } = self;
        capability.mark_values(queues);
        handler.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            capability,
            reaction_type: _,
            handler,
        } = self;
        capability.sweep_values(compactions);
        handler.sweep_values(compactions);
    }
}

impl<'a> CreateHeapData<PromiseReactionRecord<'a>, PromiseReaction<'a>> for Heap {
    fn create(&mut self, data: PromiseReactionRecord<'a>) -> PromiseReaction<'a> {
        self.promise_reaction_records.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<PromiseReactionRecord<'static>>>();
        PromiseReaction(BaseIndex::last(&self.promise_reaction_records))
    }
}
