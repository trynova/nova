// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::control_abstraction_objects::async_function_objects::await_reaction::AwaitReactionIdentifier,
        execution::Agent, types::Function,
    },
    engine::rootable::{HeapRootData, HeapRootRef, Rootable},
    heap::{indexes::BaseIndex, CreateHeapData, Heap, HeapMarkAndSweep},
};

use super::promise_capability_records::PromiseCapability;

/// \[\[Type\]\]
///
/// fulfill or reject
///
/// The \[\[Type\]\] is used when \[\[Handler\]\] is empty to allow for
/// behaviour specific to the settlement type.
#[derive(Debug, Clone, Copy)]
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
pub(crate) enum PromiseReactionHandler {
    JobCallback(Function<'static>),
    Await(AwaitReactionIdentifier),
    Empty,
}

#[derive(Debug, Clone, Copy)]
pub struct PromiseReactionRecord {
    /// \[\[Capability\]\]
    ///
    /// a PromiseCapability Record or undefined
    ///
    /// The capabilities of the promise for which this record provides a
    /// reaction handler.
    pub(crate) capability: Option<PromiseCapability>,
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
    pub(crate) handler: PromiseReactionHandler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseReaction(BaseIndex<'static, PromiseReactionRecord>);

impl PromiseReaction {
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl Index<PromiseReaction> for Agent {
    type Output = PromiseReactionRecord;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        &self.heap.promise_reaction_records[index]
    }
}

impl IndexMut<PromiseReaction> for Agent {
    fn index_mut(&mut self, index: PromiseReaction) -> &mut Self::Output {
        &mut self.heap.promise_reaction_records[index]
    }
}

impl Index<PromiseReaction> for Vec<Option<PromiseReactionRecord>> {
    type Output = PromiseReactionRecord;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        self.get(index.get_index())
            .expect("PromiseReaction out of bounds")
            .as_ref()
            .expect("PromiseReaction slot empty")
    }
}

impl IndexMut<PromiseReaction> for Vec<Option<PromiseReactionRecord>> {
    fn index_mut(&mut self, index: PromiseReaction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PromiseReaction out of bounds")
            .as_mut()
            .expect("PromiseReaction slot empty")
    }
}

impl HeapMarkAndSweep for PromiseReaction {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promise_reaction_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions
            .promise_reaction_records
            .shift_index(&mut self.0);
    }
}

impl Rootable for PromiseReaction {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::PromiseReaction(value))
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

impl HeapMarkAndSweep for PromiseReactionRecord {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.capability.mark_values(queues);
        if let PromiseReactionHandler::JobCallback(_) = self.handler {
            todo!();
        }
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.capability.sweep_values(compactions);
        if let PromiseReactionHandler::JobCallback(_) = self.handler {
            todo!();
        }
    }
}

impl CreateHeapData<PromiseReactionRecord, PromiseReaction> for Heap {
    fn create(&mut self, data: PromiseReactionRecord) -> PromiseReaction {
        self.promise_reaction_records.push(Some(data));
        PromiseReaction(BaseIndex::last(&self.promise_reaction_records))
    }
}
