// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;

use crate::{
    ecmascript::{
        builtins::{
            ArrayBuffer,
            array_buffer::{ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
            shared_array_buffer::SharedArrayBuffer,
            typed_array::{SharedVoidArray, VoidArray},
        },
        types::OrdinaryObject,
    },
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct TypedArrayArrayLength(pub u32);

impl core::fmt::Debug for TypedArrayArrayLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_auto() {
            f.write_str("AUTO")
        } else if self.is_overflowing() {
            f.write_str("> u32::MAX - 2")
        } else {
            self.0.fmt(f)
        }
    }
}

impl TypedArrayArrayLength {
    pub(crate) const fn value(value: u32) -> Self {
        Self(value)
    }

    pub(crate) const fn is_overflowing(self) -> bool {
        self.0 == Self::heap().0
    }

    pub(crate) const fn is_auto(self) -> bool {
        self.0 == Self::auto().0
    }

    /// A sentinel value of `u32::MAX - 1` means that the byte length is stored in an
    /// associated map in the heap. This will most likely be a very rare case,
    /// only applicable for 4GB+ buffers.
    pub(crate) const fn heap() -> Self {
        Self(u32::MAX - 1)
    }

    /// A sentinel value of `u32::MAX` means that the byte length is the
    /// `AUTO` value used in the spec.
    pub(crate) const fn auto() -> Self {
        Self(u32::MAX)
    }
}

impl Default for TypedArrayArrayLength {
    fn default() -> Self {
        Self::auto()
    }
}

impl From<Option<usize>> for TypedArrayArrayLength {
    fn from(value: Option<usize>) -> Self {
        match value {
            Some(value) => value.into(),
            None => Self::auto(),
        }
    }
}

impl From<usize> for TypedArrayArrayLength {
    fn from(value: usize) -> Self {
        if value >= Self::heap().0 as usize {
            Self::heap()
        } else {
            Self::value(value as u32)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypedArrayRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) viewed_array_buffer: ArrayBuffer<'a>,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_length: ViewedArrayBufferByteLength,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_offset: ViewedArrayBufferByteOffset,
    /// ### [\[\[ArrayLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) array_length: TypedArrayArrayLength,
}
bindable_handle!(TypedArrayRecord);

impl<'a> TypedArrayRecord<'a> {
    /// Get the byte offset value of this TypedArrayRecord.
    #[inline(always)]
    pub(crate) fn get_byte_offset(
        &self,
        key: VoidArray,
        byte_offsets: &AHashMap<VoidArray, usize>,
    ) -> usize {
        let byte_offset = self.byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *byte_offsets.get(&key).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    /// Get the byte length value of this TypedArrayRecord, or None if it is
    /// AUTO.
    #[inline(always)]
    pub(crate) fn get_byte_length(
        &self,
        key: VoidArray,
        byte_lengths: &AHashMap<VoidArray, usize>,
    ) -> Option<usize> {
        let byte_length = self.byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(*byte_lengths.get(&key).unwrap())
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }
}

impl Default for TypedArrayRecord<'_> {
    fn default() -> Self {
        Self {
            object_index: Default::default(),
            viewed_array_buffer: ArrayBuffer::_def(),
            byte_length: Default::default(),
            byte_offset: Default::default(),
            array_length: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedTypedArrayRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) viewed_array_buffer: SharedArrayBuffer<'a>,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_length: ViewedArrayBufferByteLength,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_offset: ViewedArrayBufferByteOffset,
    /// ### [\[\[ArrayLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) array_length: TypedArrayArrayLength,
}
bindable_handle!(SharedTypedArrayRecord);

impl<'a> SharedTypedArrayRecord<'a> {
    /// Get the byte offset value of this SharedTypedArrayRecord.
    #[inline(always)]
    pub(crate) fn get_byte_offset(
        &self,
        key: SharedVoidArray,
        byte_offsets: &AHashMap<SharedVoidArray, usize>,
    ) -> usize {
        let byte_offset = self.byte_offset;
        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            *byte_offsets.get(&key).unwrap()
        } else {
            byte_offset.0 as usize
        }
    }

    /// Get the byte length value of this SharedTypedArrayRecord, or None if it
    /// is AUTO.
    #[inline(always)]
    pub(crate) fn get_byte_length(
        &self,
        key: SharedVoidArray,
        byte_lengths: &AHashMap<SharedVoidArray, usize>,
    ) -> Option<usize> {
        let byte_length = self.byte_length;
        if byte_length == ViewedArrayBufferByteLength::heap() {
            Some(*byte_lengths.get(&key).unwrap())
        } else if byte_length == ViewedArrayBufferByteLength::auto() {
            None
        } else {
            Some(byte_length.0 as usize)
        }
    }
}

impl Default for SharedTypedArrayRecord<'_> {
    fn default() -> Self {
        Self {
            object_index: Default::default(),
            viewed_array_buffer: SharedArrayBuffer::_DEF,
            byte_length: Default::default(),
            byte_offset: Default::default(),
            array_length: Default::default(),
        }
    }
}

impl HeapMarkAndSweep for TypedArrayRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            viewed_array_buffer,
            byte_length: _,
            byte_offset: _,
            array_length: _,
        } = self;
        object_index.mark_values(queues);
        viewed_array_buffer.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            viewed_array_buffer,
            byte_length: _,
            byte_offset: _,
            array_length: _,
        } = self;
        object_index.sweep_values(compactions);
        viewed_array_buffer.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for SharedTypedArrayRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            viewed_array_buffer,
            byte_length: _,
            byte_offset: _,
            array_length: _,
        } = self;
        object_index.mark_values(queues);
        viewed_array_buffer.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            viewed_array_buffer,
            byte_length: _,
            byte_offset: _,
            array_length: _,
        } = self;
        object_index.sweep_values(compactions);
        viewed_array_buffer.sweep_values(compactions);
    }
}
