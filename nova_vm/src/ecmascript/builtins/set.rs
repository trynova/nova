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

use self::data::{SetHeapData, SetHeapDataMut, SetHeapDataRef};
use soavec::SoAVec;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Set<'a>(BaseIndex<'a, SetHeapData<'static>>);

impl<'gc> Set<'gc> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    #[inline(always)]
    pub(crate) fn get<'a>(self, agent: &'a Agent) -> SetHeapDataRef<'a, 'gc> {
        self.get_direct(&agent.heap.sets)
    }

    #[inline(always)]
    pub(crate) fn get_mut<'a>(self, agent: &'a mut Agent) -> SetHeapDataMut<'a, 'gc> {
        self.get_direct_mut(&mut agent.heap.sets)
    }

    #[inline(always)]
    pub(crate) fn get_direct<'a>(
        self,
        sets: &'a SoAVec<SetHeapData<'static>>,
    ) -> SetHeapDataRef<'a, 'gc> {
        sets.get(self.0.into_u32_index())
            .expect("Invalid Set reference")
    }

    #[inline(always)]
    pub(crate) fn get_direct_mut<'a>(
        self,
        sets: &'a mut SoAVec<SetHeapData<'static>>,
    ) -> SetHeapDataMut<'a, 'gc> {
        // SAFETY: Lifetime transmute to thread GC lifetime to temporary heap
        // reference.
        unsafe {
            core::mem::transmute::<SetHeapDataMut<'a, 'static>, SetHeapDataMut<'a, 'gc>>(
                sets.get_mut(self.0.into_u32_index())
                    .expect("Invalid Set reference"),
            )
        }
    }
}

bindable_handle!(Set);

impl<'a> From<Set<'a>> for Object<'a> {
    fn from(value: Set<'a>) -> Self {
        Object::Set(value)
    }
}

impl<'a> TryFrom<Value<'a>> for Set<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        if let Value::Set(set) = value {
            Ok(set)
        } else {
            Err(())
        }
    }
}

impl<'a> TryFrom<Object<'a>> for Set<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        if let Object::Set(set) = value {
            Ok(set)
        } else {
            Err(())
        }
    }
}

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

impl TryFrom<HeapRootData> for Set<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::Set(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> CreateHeapData<SetHeapData<'a>, Set<'a>> for Heap {
    fn create(&mut self, data: SetHeapData<'a>) -> Set<'a> {
        let i = self.sets.len();
        self.sets
            .push(data.unbind())
            .expect("Failed to allocate Set");
        self.alloc_counter += core::mem::size_of::<SetHeapData<'static>>();
        Set(BaseIndex::from_u32_index(i))
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
