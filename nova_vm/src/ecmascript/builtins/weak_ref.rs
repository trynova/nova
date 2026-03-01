// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub(crate) use data::*;

use crate::{
    ecmascript::{
        Agent, InternalMethods, InternalSlots, OrdinaryObject, ProtoIntrinsics, WeakKey,
        object_handle,
    },
    engine::Bindable,
    heap::{
        ArenaAccess, ArenaAccessMut, BaseIndex, CompactionLists, CreateHeapData, Heap,
        HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access,
    },
};

/// ## [26.1 WeakRef Objects](https://tc39.es/ecma262/#sec-weak-ref-objects)
///
/// A WeakRef is an object that is used to refer to a target object or symbol
/// without preserving it from garbage collection. WeakRefs can be dereferenced
/// to allow access to the target value, if the target hasn't been reclaimed by
/// garbage collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct WeakRef<'a>(BaseIndex<'a, WeakRefHeapData<'static>>);
object_handle!(WeakRef);
arena_vec_access!(WeakRef, 'a, WeakRefHeapData, weak_refs);

impl<'a> WeakRef<'a> {
    pub(crate) fn set_target(self, agent: &mut Agent, target: WeakKey) {
        self.get_mut(agent).weak_ref_target = Some(target.unbind());
        // Note: WeakRefTarget is set only from the constructor, and it also
        // adds the WeakRef into the [[KeptAlive]] list; hence we set the
        // boolean here.
        self.get_mut(agent).kept_alive = true;
    }

    pub(crate) fn get_target(self, agent: &mut Agent) -> Option<WeakKey<'a>> {
        let target = self.get(agent).weak_ref_target;
        if target.is_some() {
            // When observed, WeakRef gets added to [[KeptAlive]] list.
            self.get_mut(agent).kept_alive = true;
        }
        target
    }
}

impl<'a> InternalSlots<'a> for WeakRef<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakRef;

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

impl<'a> InternalMethods<'a> for WeakRef<'a> {}

impl<'a> CreateHeapData<WeakRefHeapData<'a>, WeakRef<'a>> for Heap {
    fn create(&mut self, data: WeakRefHeapData<'a>) -> WeakRef<'a> {
        self.weak_refs.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<WeakRefHeapData<'static>>();
        WeakRef(BaseIndex::last(&self.weak_refs))
    }
}

impl HeapMarkAndSweep for WeakRef<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.weak_refs.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.weak_refs.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for WeakRef<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.weak_refs.shift_weak_index(self.0).map(Self)
    }
}
