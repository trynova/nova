// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [25.1 ArrayBuffer Objects](https://tc39.es/ecma262/#sec-arraybuffer-objects)

mod abstract_operations;
mod data;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::types::SHARED_ARRAY_BUFFER_DISCRIMINANT;
use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            ARRAY_BUFFER_DISCRIMINANT, InternalMethods, InternalSlots, Object, OrdinaryObject,
            Value, Viewable, copy_data_block_bytes, create_byte_data_block,
        },
    },
    engine::{
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

use abstract_operations::detach_array_buffer;
pub(crate) use abstract_operations::*;
pub use data::*;
use ecmascript_atomics::Ordering;

#[cfg(feature = "shared-array-buffer")]
use super::shared_array_buffer::SharedArrayBuffer;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ArrayBuffer<'a>(BaseIndex<'a, ArrayBufferHeapData<'static>>);
array_buffer_handle!(ArrayBuffer);

impl ArrayBuffer<'_> {
    pub fn new<'gc>(
        agent: &mut Agent,
        byte_length: usize,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, ArrayBuffer<'gc>> {
        let block = create_byte_data_block(agent, byte_length as u64, gc)?;
        Ok(agent
            .heap
            .create(ArrayBufferHeapData::new_fixed_length(block))
            .bind(gc))
    }

    #[inline]
    pub fn is_detached(self, agent: &Agent) -> bool {
        agent[self].is_detached()
    }

    #[inline]
    pub fn is_resizable(self, agent: &Agent) -> bool {
        agent[self].is_resizable()
    }

    #[inline]
    pub fn byte_length(self, agent: &Agent) -> usize {
        agent[self].byte_length()
    }

    #[inline]
    pub fn max_byte_length(self, agent: &Agent) -> usize {
        agent[self].max_byte_length()
    }

    #[inline]
    pub fn get_detach_key(self, agent: &Agent) -> Option<DetachKey> {
        agent.heap.array_buffer_detach_keys.get(&self).copied()
    }

    #[inline]
    pub fn set_detach_key(self, agent: &mut Agent, key: Option<DetachKey>) {
        if let Some(key) = key {
            agent.heap.alloc_counter += core::mem::size_of::<(ArrayBuffer, DetachKey)>();
            agent
                .heap
                .array_buffer_detach_keys
                .insert(self.unbind(), key);
        } else {
            agent.heap.array_buffer_detach_keys.remove(&self.unbind());
        }
    }

    pub fn detach<'a>(
        self,
        agent: &mut Agent,
        key: Option<DetachKey>,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        detach_array_buffer(agent, self, key, gc)
    }

    /// Resize a Resizable ArrayBuffer.
    ///
    /// `new_byte_length` must be a safe integer.
    pub(crate) fn resize(self, agent: &mut Agent, new_byte_length: usize) {
        agent[self].resize(new_byte_length);
    }

    /// Get temporary access to an ArrayBuffer's backing data block as a slice
    /// of bytes. The access can only be held while all JavaScript is paused.
    ///
    /// ## Safety
    ///
    /// The function itself has no safety implications, but the caller should
    /// keep in mind that if JavaScript is called into the contents of the
    /// ArrayBuffer may be rewritten or reallocated.
    #[inline]
    pub fn as_slice(self, agent: &Agent) -> &[u8] {
        agent[self].get_data_block()
    }

    /// Get temporary exclusive access to an ArrayBuffer's backing data block
    /// as a slice of bytes. The access can only be held while all JavaScript
    /// is paused.
    ///
    /// ## Safety
    ///
    /// The function itself has no safety implications, but the caller should
    /// keep in mind that if JavaScript is called into the contents of the
    /// ArrayBuffer may be rewritten or reallocated.
    #[inline]
    pub fn as_mut_slice(self, agent: &mut Agent) -> &mut [u8] {
        agent[self].get_data_block_mut()
    }

    /// Create a T slice from an ArrayBuffer and byte offset and length values.
    ///
    /// This method should be used when looping over items of a TypedArray.
    pub(crate) fn as_viewable_slice<T: Viewable>(
        self,
        agent: &Agent,
        byte_offset: usize,
        byte_length: Option<usize>,
    ) -> &[T] {
        let byte_slice = self.as_slice(agent);
        let byte_limit = byte_length.map(|byte_length| byte_offset.saturating_add(byte_length));
        if byte_limit.unwrap_or(byte_offset) > byte_slice.len() {
            return &[];
        }
        let byte_slice = if let Some(byte_limit) = byte_limit {
            &byte_slice[byte_offset..byte_limit]
        } else {
            &byte_slice[byte_offset..]
        };
        // SAFETY: All bytes in byte_slice are initialized, and all bitwise
        // combinations of T are valid values. Alignment of T's is
        // guaranteed by align_to_mut itself.
        let (head, slice, _) = unsafe { byte_slice.align_to::<T>() };
        if !head.is_empty() {
            panic!("ArrayBuffer is not properly aligned for T");
        }
        slice
    }

    /// Create a T slice from an ArrayBuffer and byte offset and length values.
    ///
    /// This method should be used when looping over items of a TypedArray.
    pub(crate) fn as_mut_viewable_slice<T: Viewable>(
        self,
        agent: &mut Agent,
        byte_offset: usize,
        byte_length: Option<usize>,
    ) -> &mut [T] {
        let byte_slice = self.as_mut_slice(agent);
        let byte_limit = byte_length.map(|byte_length| byte_offset.saturating_add(byte_length));
        if byte_limit.unwrap_or(byte_offset) > byte_slice.len() {
            return &mut [];
        }
        let byte_slice = if let Some(byte_limit) = byte_limit {
            &mut byte_slice[byte_offset..byte_limit]
        } else {
            &mut byte_slice[byte_offset..]
        };
        // SAFETY: All bytes in byte_slice are initialized, and all bitwise
        // combinations of T are valid values. Alignment of T's is
        // guaranteed by align_to_mut itself.
        let (head, slice, _) = unsafe { byte_slice.align_to_mut::<T>() };
        if !head.is_empty() {
            panic!("ArrayBuffer is not properly aligned for T");
        }
        slice
    }

    /// Copy data from `source` ArrayBuffer to this ArrayBuffer.
    ///
    /// `self` and `source` must be different ArrayBuffers.
    pub(crate) fn copy_array_buffer_data(
        self,
        agent: &mut Agent,
        source: ArrayBuffer,
        first: usize,
        count: usize,
    ) {
        debug_assert_ne!(self, source);
        let array_buffers = &mut *agent.heap.array_buffers;
        let (source_data, target_data) = if self.get_index() > source.get_index() {
            let (before, after) = array_buffers.split_at_mut(self.get_index());
            (&before[source.get_index()], &mut after[0])
        } else {
            let (before, after) = array_buffers.split_at_mut(source.get_index());
            (&after[0], &mut before[self.get_index()])
        };
        let source_data = source_data.buffer.get_data_block();
        let target_data = target_data.buffer.get_data_block_mut();
        copy_data_block_bytes(target_data, 0, source_data, first, count);
    }
}

impl<'a> InternalSlots<'a> for ArrayBuffer<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::ArrayBuffer;

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

impl<'a> InternalMethods<'a> for ArrayBuffer<'a> {}

impl HeapMarkAndSweep for ArrayBuffer<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.array_buffers.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.array_buffers.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for ArrayBuffer<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.array_buffers.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<ArrayBufferHeapData<'a>, ArrayBuffer<'a>> for Heap {
    fn create(&mut self, data: ArrayBufferHeapData<'a>) -> ArrayBuffer<'a> {
        self.array_buffers.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<ArrayBufferHeapData<'static>>();
        ArrayBuffer(BaseIndex::last(&self.array_buffers))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AnyArrayBuffer<'a> {
    ArrayBuffer(ArrayBuffer<'a>) = ARRAY_BUFFER_DISCRIMINANT,
    #[cfg(feature = "shared-array-buffer")]
    SharedArrayBuffer(SharedArrayBuffer<'a>) = SHARED_ARRAY_BUFFER_DISCRIMINANT,
}
bindable_handle!(AnyArrayBuffer);

macro_rules! array_buffer_delegate {
    ($value: ident, $method: ident, $($arg:expr),*) => {
        match $value {
            Self::ArrayBuffer(ta) => ta.$method($($arg),+),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sta) => sta.$method($($arg),+),
        }
    };
}

impl<'ab> AnyArrayBuffer<'ab> {
    /// Returns true if the ArrayBuffer is a SharedArrayBuffer.
    #[inline(always)]
    pub fn is_shared(self) -> bool {
        match self {
            Self::ArrayBuffer(_) => false,
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(_) => true,
        }
    }

    #[inline(always)]
    pub fn is_detached(self, agent: &Agent) -> bool {
        array_buffer_delegate!(self, is_detached, agent)
    }

    /// Returns true if the ArrayBuffer is resizable.
    #[inline(always)]
    pub fn is_resizable(self, agent: &Agent) -> bool {
        match self {
            Self::ArrayBuffer(ta) => ta.is_resizable(agent),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sta) => sta.is_growable(agent),
        }
    }

    /// \[\[ByteLength]]
    #[inline(always)]
    pub fn byte_length(self, agent: &Agent, order: Ordering) -> usize {
        #[cfg(not(feature = "shared-array-buffer"))]
        let _ = order;
        match self {
            Self::ArrayBuffer(ta) => ta.byte_length(agent),
            #[cfg(feature = "shared-array-buffer")]
            Self::SharedArrayBuffer(sta) => sta.byte_length(agent, order),
        }
    }
}

impl<'a> From<AnyArrayBuffer<'a>> for Object<'a> {
    #[inline(always)]
    fn from(value: AnyArrayBuffer<'a>) -> Self {
        match value {
            AnyArrayBuffer::ArrayBuffer(dv) => Self::ArrayBuffer(dv),
            #[cfg(feature = "shared-array-buffer")]
            AnyArrayBuffer::SharedArrayBuffer(sdv) => Self::SharedArrayBuffer(sdv),
        }
    }
}

impl<'a> TryFrom<Value<'a>> for AnyArrayBuffer<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::ArrayBuffer(ab) => Ok(Self::ArrayBuffer(ab)),
            #[cfg(feature = "shared-array-buffer")]
            Value::SharedArrayBuffer(sab) => Ok(Self::SharedArrayBuffer(sab)),
            _ => Err(()),
        }
    }
}

impl TryFrom<HeapRootData> for AnyArrayBuffer<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::ArrayBuffer(dv) => Ok(AnyArrayBuffer::ArrayBuffer(dv)),
            #[cfg(feature = "shared-array-buffer")]
            HeapRootData::SharedArrayBuffer(sdv) => Ok(AnyArrayBuffer::SharedArrayBuffer(sdv)),
            _ => Err(()),
        }
    }
}

macro_rules! array_buffer_handle {
    ($name: ident) => {
        crate::ecmascript::types::object_handle!($name);

        impl<'a> From<$name<'a>> for crate::ecmascript::builtins::array_buffer::AnyArrayBuffer<'a> {
            fn from(value: $name<'a>) -> Self {
                Self::$name(value)
            }
        }

        impl<'a> TryFrom<crate::ecmascript::builtins::array_buffer::AnyArrayBuffer<'a>>
            for $name<'a>
        {
            type Error = ();

            fn try_from(
                value: crate::ecmascript::builtins::array_buffer::AnyArrayBuffer<'a>,
            ) -> Result<Self, Self::Error> {
                match value {
                    crate::ecmascript::builtins::array_buffer::AnyArrayBuffer::$name(data) => {
                        Ok(data)
                    }
                    _ => Err(()),
                }
            }
        }
    };
}
pub(crate) use array_buffer_handle;
