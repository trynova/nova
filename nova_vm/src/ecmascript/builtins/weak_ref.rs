// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics, WeakKey},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::WeakRefHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakRef<'a>(BaseIndex<'a, WeakRefHeapData<'static>>);

impl<'a> WeakRef<'a> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn set_target(self, agent: &mut Agent, target: WeakKey) {
        agent[self].weak_ref_target = Some(target.unbind());
        // Note: WeakRefTarget is set only from the constructor, and it also
        // adds the WeakRef into the [[KeptAlive]] list; hence we set the
        // boolean here.
        agent[self].kept_alive = true;
    }

    pub(crate) fn get_target(self, agent: &mut Agent) -> Option<WeakKey<'a>> {
        let target = agent[self].weak_ref_target;
        if target.is_some() {
            // When observed, WeakRef gets added to [[KeptAlive]] list.
            agent[self].kept_alive = true;
        }
        target
    }
}

bindable_handle!(WeakRef);

impl<'a> From<WeakRef<'a>> for Value<'a> {
    fn from(value: WeakRef<'a>) -> Self {
        Value::WeakRef(value)
    }
}

impl<'a> From<WeakRef<'a>> for Object<'a> {
    fn from(value: WeakRef<'a>) -> Self {
        Object::WeakRef(value)
    }
}

impl<'a> InternalSlots<'a> for WeakRef<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakRef;

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

impl<'a> InternalMethods<'a> for WeakRef<'a> {}

impl Index<WeakRef<'_>> for Agent {
    type Output = WeakRefHeapData<'static>;

    fn index(&self, index: WeakRef) -> &Self::Output {
        &self.heap.weak_refs[index]
    }
}

impl IndexMut<WeakRef<'_>> for Agent {
    fn index_mut(&mut self, index: WeakRef) -> &mut Self::Output {
        &mut self.heap.weak_refs[index]
    }
}

impl Index<WeakRef<'_>> for Vec<Option<WeakRefHeapData<'static>>> {
    type Output = WeakRefHeapData<'static>;

    fn index(&self, index: WeakRef) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakRef out of bounds")
            .as_ref()
            .expect("WeakRef slot empty")
    }
}

impl IndexMut<WeakRef<'_>> for Vec<Option<WeakRefHeapData<'static>>> {
    fn index_mut(&mut self, index: WeakRef) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakRef out of bounds")
            .as_mut()
            .expect("WeakRef slot empty")
    }
}

impl TryFrom<HeapRootData> for WeakRef<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::WeakRef(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> CreateHeapData<WeakRefHeapData<'a>, WeakRef<'a>> for Heap {
    fn create(&mut self, data: WeakRefHeapData<'a>) -> WeakRef<'a> {
        self.weak_refs.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<WeakRefHeapData<'static>>>();
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
