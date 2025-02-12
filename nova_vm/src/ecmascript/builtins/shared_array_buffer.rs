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
    engine::{context::NoGcScope, rootable::HeapRootData, Scoped},
    heap::{
        indexes::SharedArrayBufferIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

use self::data::SharedArrayBufferHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SharedArrayBuffer<'a>(pub(crate) SharedArrayBufferIndex<'a>);

impl SharedArrayBuffer<'_> {
    /// Unbind this SharedArrayBuffer from its current lifetime. This is necessary to use
    /// the SharedArrayBuffer as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> SharedArrayBuffer<'static> {
        unsafe { core::mem::transmute::<Self, SharedArrayBuffer<'static>>(self) }
    }

    // Bind this SharedArrayBuffer to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your SharedArrayBuffers cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let shared_array_buffer = shared_array_buffer.bind(&gc);
    // ```
    // to make sure that the unbound SharedArrayBuffer cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> SharedArrayBuffer<'gc> {
        unsafe { core::mem::transmute::<Self, SharedArrayBuffer<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, SharedArrayBuffer<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        SharedArrayBuffer(SharedArrayBufferIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for SharedArrayBuffer<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for SharedArrayBuffer<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl From<SharedArrayBuffer<'_>> for Value {
    fn from(val: SharedArrayBuffer) -> Self {
        Value::SharedArrayBuffer(val.unbind())
    }
}

impl<'a> From<SharedArrayBuffer<'a>> for Object<'a> {
    fn from(val: SharedArrayBuffer) -> Self {
        Object::SharedArrayBuffer(val.unbind())
    }
}

impl Index<SharedArrayBuffer<'_>> for Agent {
    type Output = SharedArrayBufferHeapData;

    fn index(&self, index: SharedArrayBuffer) -> &Self::Output {
        &self.heap.shared_array_buffers[index]
    }
}

impl IndexMut<SharedArrayBuffer<'_>> for Agent {
    fn index_mut(&mut self, index: SharedArrayBuffer) -> &mut Self::Output {
        &mut self.heap.shared_array_buffers[index]
    }
}

impl Index<SharedArrayBuffer<'_>> for Vec<Option<SharedArrayBufferHeapData>> {
    type Output = SharedArrayBufferHeapData;

    fn index(&self, index: SharedArrayBuffer) -> &Self::Output {
        self.get(index.get_index())
            .expect("SharedArrayBuffer out of bounds")
            .as_ref()
            .expect("SharedArrayBuffer slot empty")
    }
}

impl IndexMut<SharedArrayBuffer<'_>> for Vec<Option<SharedArrayBufferHeapData>> {
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
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
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

impl CreateHeapData<SharedArrayBufferHeapData, SharedArrayBuffer<'static>> for Heap {
    fn create(&mut self, data: SharedArrayBufferHeapData) -> SharedArrayBuffer<'static> {
        self.shared_array_buffers.push(Some(data));
        SharedArrayBuffer(SharedArrayBufferIndex::last(&self.shared_array_buffers))
    }
}
