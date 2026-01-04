// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            set::Set,
        },
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, OrdinaryObject, object_handle},
    },
    engine::context::{Bindable, bindable_handle},
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, arena_vec_access,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SetIterator<'a>(BaseIndex<'a, SetIteratorHeapData<'static>>);
object_handle!(SetIterator);
arena_vec_access!(SetIterator, 'a, SetIteratorHeapData, set_iterators);

impl SetIterator<'_> {
    pub(crate) fn from_set(agent: &mut Agent, set: Set, kind: CollectionIteratorKind) -> Self {
        agent.heap.create(SetIteratorHeapData {
            object_index: None,
            set: Some(set.unbind()),
            next_index: 0,
            kind,
        })
    }
}

impl<'a> InternalSlots<'a> for SetIterator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SetIterator;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for SetIterator<'a> {}

impl<'a> CreateHeapData<SetIteratorHeapData<'a>, SetIterator<'a>> for Heap {
    fn create(&mut self, data: SetIteratorHeapData<'a>) -> SetIterator<'a> {
        self.set_iterators.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<SetIteratorHeapData<'static>>();
        SetIterator(BaseIndex::last(&self.set_iterators))
    }
}

impl HeapMarkAndSweep for SetIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.set_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.set_iterators.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for SetIterator<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.set_iterators.shift_weak_index(self.0).map(Self)
    }
}

bindable_handle!(SetIteratorHeapData);

#[derive(Debug, Clone, Copy, Default)]
pub struct SetIteratorHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) set: Option<Set<'a>>,
    pub(crate) next_index: usize,
    pub(crate) kind: CollectionIteratorKind,
}

impl HeapMarkAndSweep for SetIteratorHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            set,
            next_index: _,
            kind: _,
        } = self;
        object_index.mark_values(queues);
        set.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            set,
            next_index: _,
            kind: _,
        } = self;
        object_index.sweep_values(compactions);
        set.sweep_values(compactions);
    }
}
