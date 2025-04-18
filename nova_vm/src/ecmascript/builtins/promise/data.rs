// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::control_abstraction_objects::promise_objects::promise_abstract_operations::{
            promise_jobs::new_promise_reaction_job, promise_reaction_records::PromiseReaction,
        },
        execution::Agent,
        types::{OrdinaryObject, Value},
    },
    engine::context::NoGcScope,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct PromiseHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) promise_state: PromiseState<'a>,
}

#[derive(Debug, Clone)]
pub(crate) enum PromiseState<'a> {
    Pending {
        fulfill_reactions: Option<PromiseReactions<'a>>,
        reject_reactions: Option<PromiseReactions<'a>>,
        /// True if the resolution state of this promise depends on another
        /// promise or thenable that hasn't fulfilled or rejected yet.
        is_resolved: bool,
    },
    Fulfilled {
        promise_result: Value<'a>,
    },
    Rejected {
        promise_result: Value<'a>,
        is_handled: bool,
    },
}

impl Default for PromiseState<'_> {
    fn default() -> Self {
        Self::Pending {
            fulfill_reactions: None,
            reject_reactions: None,
            is_resolved: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum PromiseReactions<'a> {
    One(PromiseReaction<'a>),
    Many(Vec<PromiseReaction<'a>>),
}

impl PromiseReactions<'_> {
    /// ### [27.2.1.8 TriggerPromiseReactions ( reactions, argument )](https://tc39.es/ecma262/#sec-triggerpromisereactions)
    pub(crate) fn trigger(&self, agent: &mut Agent, argument: Value, gc: NoGcScope) {
        match self {
            PromiseReactions::One(reaction) => {
                let job = new_promise_reaction_job(agent, *reaction, argument, gc);
                agent.host_hooks.enqueue_promise_job(job);
            }
            PromiseReactions::Many(vec) => {
                for reaction in vec {
                    let job = new_promise_reaction_job(agent, *reaction, argument, gc);
                    agent.host_hooks.enqueue_promise_job(job);
                }
            }
        };
    }
}

impl HeapMarkAndSweep for PromiseReactions<'static> {
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

impl HeapMarkAndSweep for PromiseHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            promise_state,
        } = self;
        object_index.mark_values(queues);
        promise_state.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            promise_state,
        } = self;
        object_index.sweep_values(compactions);
        promise_state.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for PromiseState<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            PromiseState::Pending {
                fulfill_reactions,
                reject_reactions,
                is_resolved: _,
            } => {
                fulfill_reactions.mark_values(queues);
                reject_reactions.mark_values(queues);
            }
            PromiseState::Fulfilled { promise_result }
            | PromiseState::Rejected {
                promise_result,
                is_handled: _,
            } => {
                promise_result.mark_values(queues);
            }
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            PromiseState::Pending {
                fulfill_reactions,
                reject_reactions,
                is_resolved: _,
            } => {
                fulfill_reactions.sweep_values(compactions);
                reject_reactions.sweep_values(compactions);
            }
            PromiseState::Fulfilled { promise_result }
            | PromiseState::Rejected {
                promise_result,
                is_handled: _,
            } => {
                promise_result.sweep_values(compactions);
            }
        }
    }
}
