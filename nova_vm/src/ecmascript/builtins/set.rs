// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub(crate) use data::*;

use crate::{
    Heap,
    ecmascript::{
        Agent, ProtoIntrinsics,
        InternalMethods, InternalSlots, OrdinaryObject, object_handle,
    },
    engine::context::Bindable,
    heap::{
        ArenaAccessSoA, ArenaAccessSoAMut, CompactionLists, CreateHeapData, HeapMarkAndSweep,
        HeapSweepWeakReference, WorkQueues, arena_vec_access,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Set<'a>(BaseIndex<'a, SetHeapData<'static>>);
object_handle!(Set);
arena_vec_access!(soa: Set, 'a, SetHeapData, sets, SetHeapDataRef, SetHeapDataMut);

impl<'gc> Set<'gc> {}

impl<'a> InternalSlots<'a> for Set<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Set;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for Set<'a> {}

impl HeapMarkAndSweep for Set<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.sets.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.sets.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Set<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.sets.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<SetHeapData<'a>, Set<'a>> for Heap {
    fn create(&mut self, data: SetHeapData<'a>) -> Set<'a> {
        let i = self.sets.len();
        self.sets
            .push(data.unbind())
            .expect("Failed to allocate Set");
        self.alloc_counter += core::mem::size_of::<SetHeapData<'static>>();
        Set(BaseIndex::from_index_u32(i))
    }
}

impl HeapMarkAndSweep for SetHeapDataRef<'_, 'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            set_data: _,
            values,
            object_index,
            needs_primitive_rehashing: _,
        } = self;
        values.mark_values(queues);
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, _: &CompactionLists) {
        unreachable!()
    }
}

impl HeapMarkAndSweep for SetHeapDataMut<'_, 'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            set_data: _,
            values,
            object_index,
            needs_primitive_rehashing: _,
        } = self;
        values.mark_values(queues);
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            set_data: _,
            values,
            object_index,
            needs_primitive_rehashing: _,
        } = self;
        values.sweep_values(compactions);
        object_index.sweep_values(compactions);
    }
}
