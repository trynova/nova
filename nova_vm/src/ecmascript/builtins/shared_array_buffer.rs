// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{
        context::{Bindable, NoGcScope},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
        indexes::SharedArrayBufferIndex,
    },
};

use self::data::SharedArrayBufferHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SharedArrayBuffer<'a>(pub(crate) SharedArrayBufferIndex<'a>);

impl SharedArrayBuffer<'_> {
    pub(crate) const fn _def() -> Self {
        SharedArrayBuffer(SharedArrayBufferIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SharedArrayBuffer<'_> {
    type Of<'a> = SharedArrayBuffer<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> IntoValue<'a> for SharedArrayBuffer<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for SharedArrayBuffer<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<SharedArrayBuffer<'a>> for Value<'a> {
    fn from(value: SharedArrayBuffer<'a>) -> Self {
        Value::SharedArrayBuffer(value)
    }
}

impl<'a> From<SharedArrayBuffer<'a>> for Object<'a> {
    fn from(value: SharedArrayBuffer<'a>) -> Self {
        Object::SharedArrayBuffer(value)
    }
}

impl Index<SharedArrayBuffer<'_>> for Agent {
    type Output = SharedArrayBufferHeapData<'static>;

    fn index(&self, index: SharedArrayBuffer) -> &Self::Output {
        &self.heap.shared_array_buffers[index]
    }
}

impl IndexMut<SharedArrayBuffer<'_>> for Agent {
    fn index_mut(&mut self, index: SharedArrayBuffer) -> &mut Self::Output {
        &mut self.heap.shared_array_buffers[index]
    }
}

impl Index<SharedArrayBuffer<'_>> for Vec<Option<SharedArrayBufferHeapData<'static>>> {
    type Output = SharedArrayBufferHeapData<'static>;

    fn index(&self, index: SharedArrayBuffer) -> &Self::Output {
        self.get(index.get_index())
            .expect("SharedArrayBuffer out of bounds")
            .as_ref()
            .expect("SharedArrayBuffer slot empty")
    }
}

impl IndexMut<SharedArrayBuffer<'_>> for Vec<Option<SharedArrayBufferHeapData<'static>>> {
    fn index_mut(&mut self, index: SharedArrayBuffer) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("SharedArrayBuffer out of bounds")
            .as_mut()
            .expect("SharedArrayBuffer slot empty")
    }
}

impl<'a> InternalSlots<'a> for SharedArrayBuffer<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SharedArrayBuffer;

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

impl<'a> InternalMethods<'a> for SharedArrayBuffer<'a> {}

impl TryFrom<HeapRootData> for SharedArrayBuffer<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::SharedArrayBuffer(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl HeapMarkAndSweep for SharedArrayBuffer<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.shared_array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.shared_array_buffers.shift_index(&mut self.0);
    }
}

impl<'a> CreateHeapData<SharedArrayBufferHeapData<'a>, SharedArrayBuffer<'a>> for Heap {
    fn create(&mut self, data: SharedArrayBufferHeapData<'a>) -> SharedArrayBuffer<'a> {
        self.shared_array_buffers.push(Some(data.unbind()));
        #[cfg(feature = "interleaved-gc")]
        {
            self.alloc_counter +=
                core::mem::size_of::<Option<SharedArrayBufferHeapData<'static>>>();
        }
        SharedArrayBuffer(SharedArrayBufferIndex::last(&self.shared_array_buffers))
    }
}
