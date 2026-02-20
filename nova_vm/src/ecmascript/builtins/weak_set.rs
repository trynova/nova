// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub(crate) use data::*;

use crate::{
    ecmascript::{
        Agent, Behaviour, Function, InternalMethods, InternalSlots, OrdinaryObject,
        ProtoIntrinsics, WeakSetPrototype, object_handle,
    },
    engine::Bindable,
    heap::{
        ArenaAccess, ArenaAccessMut, BaseIndex, CompactionLists, CreateHeapData, Heap,
        HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct WeakSet<'a>(BaseIndex<'a, WeakSetHeapData<'static>>);
object_handle!(WeakSet);
arena_vec_access!(WeakSet, 'a, WeakSetHeapData, weak_sets);

impl WeakSet<'_> {
    /// Returns true if the function is equal to %WeakSet.prototype.add%.
    pub(crate) fn is_weak_set_prototype_add(agent: &Agent, function: Function) -> bool {
        let Function::BuiltinFunction(function) = function else {
            return false;
        };
        let Behaviour::Regular(behaviour) = function.get(agent).local().behaviour else {
            return false;
        };
        // We allow a function address comparison here against best advice: it
        // is exceedingly unlikely that the `add` function wouldn't be unique
        // and even if it isn't, we don't care since we only care about its
        // inner workings.
        #[allow(unknown_lints, renamed_and_removed_lints)]
        {
            #[allow(
                clippy::fn_address_comparisons,
                unpredictable_function_pointer_comparisons
            )]
            {
                behaviour == WeakSetPrototype::add
            }
        }
    }
}

impl<'a> InternalSlots<'a> for WeakSet<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakSet;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for WeakSet<'a> {}

impl<'a> CreateHeapData<WeakSetHeapData<'a>, WeakSet<'a>> for Heap {
    fn create(&mut self, data: WeakSetHeapData<'a>) -> WeakSet<'a> {
        self.weak_sets.push(data);
        self.alloc_counter += core::mem::size_of::<WeakSetHeapData<'static>>();
        WeakSet(BaseIndex::last(&self.weak_sets))
    }
}

impl HeapMarkAndSweep for WeakSet<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.weak_sets.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.weak_sets.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for WeakSet<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.weak_sets.shift_weak_index(self.0).map(Self)
    }
}
