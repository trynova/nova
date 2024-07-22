// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! EvalSource is a Nova-engine specific concept to capture and keep any
//! `eval(source)` source strings alive after the eval call for the case where
//! that the eval call defines functions. Those functions will refer to the
//! EvalSource for their function source text.

use std::{fmt::Debug, ops::Index, ptr::NonNull};

use oxc_allocator::Allocator;

use crate::{
    ecmascript::{execution::Agent, types::HeapString},
    heap::{
        indexes::BaseIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

type EvalSourceIndex = BaseIndex<EvalSourceHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EvalSource(EvalSourceIndex);

impl EvalSource {
    pub(crate) fn new(agent: &mut Agent, source: HeapString) -> Self {
        agent.heap.create(EvalSourceHeapData {
            source,
            allocator: NonNull::from(Box::leak(Default::default())),
        })
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn get_source_text(self, agent: &Agent) -> HeapString {
        agent[self].source
    }
}

pub(crate) struct EvalSourceHeapData {
    /// The source JavaScript string data the eval was called with. The string
    /// is known and required to be a HeapString because functions created
    /// in the eval call may keep references to the string data. If the eval
    /// string was small-string optimised and on the stack, then those
    /// references would necessarily and definitely be invalid.
    source: HeapString,
    /// The arena that contains the parsed data of the eval source.
    allocator: NonNull<Allocator>,
}

unsafe impl Send for EvalSourceHeapData {}

impl EvalSourceHeapData {
    pub(crate) fn get_allocator(&self) -> NonNull<Allocator> {
        self.allocator
    }
}

impl Debug for EvalSourceHeapData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvalSourceHeapData")
            .field("source", &self.source)
            .field("allocator", &"[binary data]")
            .finish()
    }
}

impl Drop for EvalSourceHeapData {
    fn drop(&mut self) {
        // SAFETY: All references to this EvalSource should have been dropped
        // before we drop this.
        drop(unsafe { Box::from_raw(self.allocator.as_mut()) });
    }
}

impl Index<EvalSource> for Agent {
    type Output = EvalSourceHeapData;

    fn index(&self, index: EvalSource) -> &Self::Output {
        self.heap
            .eval_sources
            .get(index.get_index())
            .expect("EvalSource out of bounds")
            .as_ref()
            .expect("EvalSource slot empty")
    }
}

impl CreateHeapData<EvalSourceHeapData, EvalSource> for Heap {
    fn create(&mut self, data: EvalSourceHeapData) -> EvalSource {
        self.eval_sources.push(Some(data));
        EvalSource(EvalSourceIndex::last(&self.eval_sources))
    }
}

impl HeapMarkAndSweep for EvalSourceHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.source.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.source.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for EvalSource {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.eval_sources.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.eval_sources.shift_index(&mut self.0);
    }
}
