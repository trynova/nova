// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub(crate) use data::*;

use crate::{
    ecmascript::{
        Agent, InternalMethods, InternalSlots, OrdinaryObject, ProtoIntrinsics, object_handle,
    },
    engine::Bindable,
    heap::{
        ArenaAccessSoA, ArenaAccessSoAMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, WorkQueues, arena_vec_access, {BaseIndex, HeapIndexHandle},
    },
};

/// ## [24.2 Set Objects](https://tc39.es/ecma262/#sec-set-objects)
///
/// _Set_ objects are collections of ECMAScript language values. A Set may
/// contain each distinct value at most once. Distinct values are discriminated
/// using the semantics of the SameValueZero comparison algorithm.
///
/// Set objects must be implemented using either hash tables or other mechanisms
/// that, on average, provide access times that are sublinear on the number of
/// elements in the collection.
///
/// ### Example
///
/// ```javascript
/// new Set()
/// ```
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
