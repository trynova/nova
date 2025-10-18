// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ecmascript_atomics::{Ordering, RacySlice};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics, agent::ExceptionType},
        types::{
            InternalMethods, InternalSlots, Object, OrdinaryObject, SharedDataBlock, Value,
            create_shared_byte_data_block,
        },
    },
    engine::{
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::SharedArrayBufferRecord;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SharedArrayBuffer<'a>(BaseIndex<'a, SharedArrayBufferRecord<'static>>);

bindable_handle!(SharedArrayBuffer);

impl<'sab> SharedArrayBuffer<'sab> {
    pub fn new<'gc>(
        agent: &mut Agent,
        byte_length: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, SharedArrayBuffer<'gc>> {
        // SAFETY: No maxByteLength.
        let block = unsafe { create_shared_byte_data_block(agent, byte_length as u64, None, gc) }?;
        Ok(agent
            .heap
            .create(SharedArrayBufferRecord::new(block, gc))
            .bind(gc))
    }

    /// Constant to be used only for creating a build-time Self.
    pub(crate) const _DEF: Self = Self(BaseIndex::ZERO);

    pub(crate) fn as_slice(self, agent: &Agent) -> RacySlice<'_, u8> {
        self.get_data_block(agent).as_racy_slice()
    }

    #[inline]
    pub fn is_detached(self, agent: &Agent) -> bool {
        self.get(agent).data_block.is_dangling()
    }

    /// Returns true if the SharedArrayBuffer is growable.
    pub fn is_growable(self, agent: &Agent) -> bool {
        self.get(agent).data_block.is_growable()
    }

    /// Get the byte length of the SharedArrayBuffer.
    ///
    /// Note, if this is a growable SharedArrayBuffer then this is a
    /// synchronising operation.
    #[inline]
    pub fn byte_length(self, agent: &Agent, order: Ordering) -> usize {
        self.get(agent).data_block.byte_length(order)
    }

    /// Get the maximum byte length of the SharedArrayBuffer.
    #[inline]
    pub fn max_byte_length(self, agent: &Agent) -> usize {
        self.get(agent).data_block.max_byte_length()
    }

    /// Get the SharedDataBlock of a SharedArrayBuffer for sharing.
    pub fn get_data_block(self, agent: &Agent) -> &SharedDataBlock {
        &self.unbind().get(agent).data_block
    }

    /// Create a new SharedArrayBuffer from a SharedDataBlock.
    pub fn new_from_data_block(
        self,
        agent: &mut Agent,
        data_block: SharedDataBlock,
        gc: NoGcScope<'sab, '_>,
    ) -> Self {
        agent
            .heap
            .create(SharedArrayBufferRecord {
                backing_object: None,
                data_block,
            })
            .bind(gc)
    }

    #[inline(always)]
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    fn get(self, agent: &Agent) -> &SharedArrayBufferRecord<'sab> {
        agent
            .heap
            .shared_array_buffers
            .get(self.get_index())
            .expect("Invalid SharedArrayBuffer")
    }

    fn get_mut(self, agent: &mut Agent) -> &mut SharedArrayBufferRecord<'static> {
        agent
            .heap
            .shared_array_buffers
            .get_mut(self.get_index())
            .expect("Invalid SharedArrayBuffer")
    }

    pub fn grow<'gc>(
        self,
        agent: &mut Agent,
        new_byte_length: u64,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        let data_block = &self.get(agent).data_block;
        let max_byte_length = data_block.max_byte_length();
        if new_byte_length > max_byte_length as u64 {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Attempted to resize beyond SharedArrayBuffer maxByteLength",
                gc,
            ));
        }
        if max_byte_length == 0 {
            // dangling.
            return Ok(());
        }
        // Note: new_byte_length is less or equal to max_byte_length which is
        // a usize.
        let new_byte_length = new_byte_length as usize;
        if unsafe { data_block.grow(new_byte_length) } {
            // Success
            Ok(())
        } else {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Attempted to shrink SharedArrayBuffer",
                gc,
            ))
        }
    }

    /// Set the SharedArrayBuffer's internal buffer to `data_block`.
    ///
    /// ## Safety
    ///
    /// The SharedArrayBuffer should not have had an internal buffer set.
    pub(crate) unsafe fn set_data_block(self, agent: &mut Agent, data_block: SharedDataBlock) {
        let data = self.get_mut(agent);
        debug_assert!(data.data_block.is_dangling());
        data.data_block = data_block;
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

impl<'a> InternalSlots<'a> for SharedArrayBuffer<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SharedArrayBuffer;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).backing_object.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .backing_object
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

impl HeapSweepWeakReference for SharedArrayBuffer<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .shared_array_buffers
            .shift_weak_index(self.0)
            .map(Self)
    }
}

impl<'a> CreateHeapData<SharedArrayBufferRecord<'a>, SharedArrayBuffer<'a>> for Heap {
    fn create(&mut self, data: SharedArrayBufferRecord<'a>) -> SharedArrayBuffer<'a> {
        self.shared_array_buffers.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<Option<SharedArrayBufferRecord<'static>>>();
        SharedArrayBuffer(BaseIndex::last_t(&self.shared_array_buffers))
    }
}
