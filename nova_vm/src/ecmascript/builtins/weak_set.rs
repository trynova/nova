// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{Function, InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

use self::data::WeakSetHeapData;

use super::{Behaviour, keyed_collections::weak_set_objects::weak_set_prototype::WeakSetPrototype};

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct WeakSet<'a>(BaseIndex<'a, WeakSetHeapData<'static>>);

impl WeakSet<'_> {
    /// Returns true if the function is equal to %WeakSet.prototype.add%.
    pub(crate) fn is_weak_set_prototype_add(agent: &Agent, function: Function) -> bool {
        let Function::BuiltinFunction(function) = function else {
            return false;
        };
        let Behaviour::Regular(behaviour) = agent[function].behaviour else {
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

bindable_handle!(WeakSet);

impl HeapIndexHandle for WeakSet<'_> {
    fn from_index_u32(index: u32) -> Self {
        Self(BaseIndex::from_index_u32(index))
    }

    fn get_index_u32(&self) -> u32 {
        self.0.get_index_u32()
    }
}

impl<'a> From<WeakSet<'a>> for Object<'a> {
    fn from(value: WeakSet<'a>) -> Self {
        Object::WeakSet(value)
    }
}

impl<'a> TryFrom<Value<'a>> for WeakSet<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::WeakSet(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for WeakSet<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakSet;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for WeakSet<'a> {}

impl TryFrom<HeapRootData> for WeakSet<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::WeakSet(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> CreateHeapData<WeakSetHeapData<'a>, WeakSet<'a>> for Heap {
    fn create(&mut self, data: WeakSetHeapData<'a>) -> WeakSet<'a> {
        self.weak_sets.push(data.unbind());
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
