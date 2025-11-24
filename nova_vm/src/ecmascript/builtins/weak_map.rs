// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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

use self::data::WeakMapRecord;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct WeakMap<'a>(BaseIndex<'a, WeakMapRecord<'static>>);

impl<'m> WeakMap<'m> {
    pub(crate) const _DEF: Self = Self(BaseIndex::from_u32_index(u32::MAX - 1));

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    fn get<'a>(self, agent: &'a Agent) -> &'a WeakMapRecord<'m> {
        self.get_direct(&agent.heap.weak_maps)
    }

    #[inline(always)]
    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut WeakMapRecord<'m> {
        self.get_direct_mut(&mut agent.heap.weak_maps)
    }

    #[inline(always)]
    fn get_direct<'a>(self, weak_maps: &'a [WeakMapRecord<'static>]) -> &'a WeakMapRecord<'m> {
        weak_maps
            .get(self.get_index())
            .expect("Invalid WeakMap reference")
    }

    #[inline(always)]
    fn get_direct_mut<'a>(
        self,
        weak_maps: &'a mut [WeakMapRecord<'static>],
    ) -> &'a mut WeakMapRecord<'m> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<&'a mut WeakMapRecord<'static>, &'a mut WeakMapRecord<'m>>(
                weak_maps
                    .get_mut(self.get_index())
                    .expect("Invalid WeakMap reference"),
            )
        }
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

impl<'a> InternalMethods<'a> for WeakMap<'a> {}

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

impl<'a> CreateHeapData<WeakMapRecord<'a>, WeakMap<'a>> for Heap {
    fn create(&mut self, data: WeakMapRecord<'a>) -> WeakMap<'a> {
        self.weak_maps.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<WeakMapRecord<'static>>();
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
