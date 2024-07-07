// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{ecmascript::{builtins::control_abstraction_objects::promise_objects::promise_abstract_operations::promise_reaction_records::PromiseReaction, types::{OrdinaryObject, Value}}, heap::{CompactionLists, HeapMarkAndSweep, WorkQueues}};

#[derive(Debug, Clone, Default)]
pub struct PromiseHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) promise_state: PromiseState,
    pub(crate) promise_fulfill_reactions: Option<PromiseReactions>,
    pub(crate) promise_reject_reactions: Option<PromiseReactions>,
    pub(crate) promise_is_handled: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum PromiseState {
    #[default]
    Pending,
    Fulfilled {
        promise_result: Value,
    },
    Rejected {
        promise_result: Value,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum PromiseReactions {
    One(PromiseReaction),
    Many(Vec<PromiseReaction>),
}

impl HeapMarkAndSweep for PromiseReactions {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            PromiseReactions::One(reaction) => reaction.mark_values(queues),
            PromiseReactions::Many(reactions) => reactions
                .iter()
                .for_each(|reaction| reaction.mark_values(queues)),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            PromiseReactions::One(reaction) => reaction.sweep_values(compactions),
            PromiseReactions::Many(reactions) => reactions
                .iter_mut()
                .for_each(|reaction| reaction.sweep_values(compactions)),
        }
    }
}

impl HeapMarkAndSweep for PromiseHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        match self.promise_state {
            PromiseState::Fulfilled { promise_result }
            | PromiseState::Rejected { promise_result } => {
                promise_result.mark_values(queues);
            }
            _ => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
