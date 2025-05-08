// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{Function, InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, NoGcScope},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues,
        indexes::{BaseIndex, WeakSetIndex},
    },
};

use self::data::WeakSetHeapData;

use super::{Behaviour, keyed_collections::weak_set_objects::weak_set_prototype::WeakSetPrototype};

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakSet<'a>(pub(crate) WeakSetIndex<'a>);

impl WeakSet<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for WeakSet<'_> {
    type Of<'a> = WeakSet<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<WeakSet<'a>> for Value<'a> {
    fn from(value: WeakSet<'a>) -> Self {
        Value::WeakSet(value)
    }
}

impl<'a> From<WeakSet<'a>> for Object<'a> {
    fn from(value: WeakSet<'a>) -> Self {
        Object::WeakSet(value)
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

impl Index<WeakSet<'_>> for Agent {
    type Output = WeakSetHeapData<'static>;

    fn index(&self, index: WeakSet) -> &Self::Output {
        &self.heap.weak_sets[index]
    }
}

impl IndexMut<WeakSet<'_>> for Agent {
    fn index_mut(&mut self, index: WeakSet) -> &mut Self::Output {
        &mut self.heap.weak_sets[index]
    }
}

impl Index<WeakSet<'_>> for Vec<Option<WeakSetHeapData<'static>>> {
    type Output = WeakSetHeapData<'static>;

    fn index(&self, index: WeakSet) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakSet out of bounds")
            .as_ref()
            .expect("WeakSet slot empty")
    }
}

impl IndexMut<WeakSet<'_>> for Vec<Option<WeakSetHeapData<'static>>> {
    fn index_mut(&mut self, index: WeakSet) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakSet out of bounds")
            .as_mut()
            .expect("WeakSet slot empty")
    }
}

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
        self.weak_sets.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<WeakSetHeapData<'static>>>();
        WeakSet(WeakSetIndex::last(&self.weak_sets))
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
