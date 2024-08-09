// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::control_abstraction_objects::async_function_objects::await_reaction::AwaitReactionIdentifier,
        execution::Agent, types::Function,
    },
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
pub(crate) enum PromiseReactionHandler<'gen> {
    JobCallback(Function<'gen>),
    Await(AwaitReactionIdentifier<'gen>),
    Empty,
}

#[derive(Debug, Clone, Copy)]
pub struct PromiseReactionRecord<'gen> {
    /// \[\[Capability\]\]
    ///
    /// a PromiseCapability Record or undefined
    ///
    /// The capabilities of the promise for which this record provides a
    /// reaction handler.
    pub(crate) capability: Option<PromiseCapability<'gen>>,
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
    pub(crate) handler: PromiseReactionHandler<'gen>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PromiseReaction<'gen>(BaseIndex<'gen, PromiseReactionRecord<'gen>>);

impl<'gen> PromiseReaction<'gen> {
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> Index<PromiseReaction<'gen>> for Agent<'gen> {
    type Output = PromiseReactionRecord<'gen>;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        &self.heap.promise_reaction_records[index]
    }
}

impl<'gen> IndexMut<PromiseReaction<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: PromiseReaction<'gen>) -> &mut Self::Output {
        &mut self.heap.promise_reaction_records[index]
    }
}

impl<'gen> Index<PromiseReaction<'gen>> for Vec<Option<PromiseReactionRecord<'gen>>> {
    type Output = PromiseReactionRecord<'gen>;

    fn index(&self, index: PromiseReaction) -> &Self::Output {
        self.get(index.get_index())
            .expect("PromiseReaction out of bounds")
            .as_ref()
            .expect("PromiseReaction slot empty")
    }
}

impl<'gen> IndexMut<PromiseReaction<'gen>> for Vec<Option<PromiseReactionRecord<'gen>>> {
    fn index_mut(&mut self, index: PromiseReaction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("PromiseReaction out of bounds")
            .as_mut()
            .expect("PromiseReaction slot empty")
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for PromiseReaction<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.promise_reaction_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions
            .promise_reaction_records
            .shift_index(&mut self.0);
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for PromiseReactionRecord<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
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

impl<'gen> CreateHeapData<PromiseReactionRecord<'gen>, PromiseReaction<'gen>> for Heap<'gen> {
    fn create(&mut self, data: PromiseReactionRecord<'gen>) -> PromiseReaction<'gen> {
        self.promise_reaction_records.push(Some(data));
        PromiseReaction(BaseIndex::last(&self.promise_reaction_records))
    }
}
