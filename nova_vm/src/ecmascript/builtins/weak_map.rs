// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    Heap,
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues,
        indexes::BaseIndex,
    },
};

use self::data::WeakMapHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakMap<'a>(BaseIndex<'a, WeakMapHeapData<'static>>);

impl WeakMap<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

bindable_handle!(WeakMap);

impl<'a> From<WeakMap<'a>> for Value<'a> {
    fn from(value: WeakMap<'a>) -> Self {
        Value::WeakMap(value)
    }
}

impl<'a> From<WeakMap<'a>> for Object<'a> {
    fn from(value: WeakMap<'a>) -> Self {
        Object::WeakMap(value)
    }
}

impl<'a> InternalSlots<'a> for WeakMap<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakMap;

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

impl<'a> InternalMethods<'a> for WeakMap<'a> {}

impl Index<WeakMap<'_>> for Agent {
    type Output = WeakMapHeapData<'static>;

    fn index(&self, index: WeakMap) -> &Self::Output {
        &self.heap.weak_maps[index]
    }
}

impl IndexMut<WeakMap<'_>> for Agent {
    fn index_mut(&mut self, index: WeakMap) -> &mut Self::Output {
        &mut self.heap.weak_maps[index]
    }
}

impl Index<WeakMap<'_>> for Vec<Option<WeakMapHeapData<'static>>> {
    type Output = WeakMapHeapData<'static>;

    fn index(&self, index: WeakMap) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakMap out of bounds")
            .as_ref()
            .expect("WeakMap slot empty")
    }
}

impl IndexMut<WeakMap<'_>> for Vec<Option<WeakMapHeapData<'static>>> {
    fn index_mut(&mut self, index: WeakMap) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakMap out of bounds")
            .as_mut()
            .expect("WeakMap slot empty")
    }
}

impl TryFrom<HeapRootData> for WeakMap<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::WeakMap(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> CreateHeapData<WeakMapHeapData<'a>, WeakMap<'a>> for Heap {
    fn create(&mut self, data: WeakMapHeapData<'a>) -> WeakMap<'a> {
        self.weak_maps.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<WeakMapHeapData<'static>>>();
        WeakMap(BaseIndex::last(&self.weak_maps))
    }
}

impl HeapMarkAndSweep for WeakMap<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.weak_maps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.weak_maps.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for WeakMap<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.weak_maps.shift_weak_index(self.0).map(Self)
    }
}
