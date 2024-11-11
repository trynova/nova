// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{builtins::ArrayBuffer, types::OrdinaryObject},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TypedArrayByteLength(pub u32);

impl TypedArrayByteLength {
    pub const fn value(value: u32) -> Self {
        Self(value)
    }

    /// A sentinel value of `u32::MAX - 1` means that the byte length is stored in an
    /// associated map in the heap. This will most likely be a very rare case,
    /// only applicable for 4GB+ buffers.
    pub const fn heap() -> Self {
        Self(u32::MAX - 1)
    }

    /// A sentinel value of `u32::MAX` means that the byte length is the
    /// `AUTO` value used in the spec.
    pub const fn auto() -> Self {
        Self(u32::MAX)
    }
}

impl Default for TypedArrayByteLength {
    fn default() -> Self {
        Self::auto()
    }
}

impl From<Option<usize>> for TypedArrayByteLength {
    fn from(value: Option<usize>) -> Self {
        match value {
            Some(value) => {
                if value >= Self::heap().0 as usize {
                    Self::heap()
                } else {
                    Self::value(value as u32)
                }
            }
            None => Self::auto(),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TypedArrayArrayLength(pub u32);

impl TypedArrayArrayLength {
    pub const fn value(value: u32) -> Self {
        Self(value)
    }

    /// A sentinel value of `u32::MAX - 1` means that the byte length is stored in an
    /// associated map in the heap. This will most likely be a very rare case,
    /// only applicable for 4GB+ buffers.
    pub const fn heap() -> Self {
        Self(u32::MAX - 1)
    }

    /// A sentinel value of `u32::MAX` means that the byte length is the
    /// `AUTO` value used in the spec.
    pub const fn auto() -> Self {
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
            Some(value) => {
                if value >= Self::heap().0 as usize {
                    Self::heap()
                } else {
                    Self::value(value as u32)
                }
            }
            None => Self::auto(),
        }
    }
}

// TODO: Investigate if the common case is that the byte offset is less than
// an u16, that would mean we could squeeze an extra 2 bytes out of the struct.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TypedArrayByteOffset(pub u32);

impl TypedArrayByteOffset {
    pub const fn value(value: u32) -> Self {
        Self(value)
    }

    /// A sentinel value of `u32::MAX` means that the byte offset is stored in
    /// an associated map in the heap. This will most likely be a very rare
    /// case, only applicable for 4GB+ buffers.
    pub const fn heap() -> Self {
        Self(u32::MAX)
    }
}

impl Default for TypedArrayByteOffset {
    fn default() -> Self {
        Self::value(0)
    }
}

impl From<usize> for TypedArrayByteOffset {
    fn from(value: usize) -> Self {
        if value >= Self::heap().0 as usize {
            Self::heap()
        } else {
            Self::value(value as u32)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypedArrayHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) viewed_array_buffer: ArrayBuffer,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_length: TypedArrayByteLength,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) byte_offset: TypedArrayByteOffset,
    /// ### [\[\[ArrayLength\]\]](https://tc39.es/ecma262/#sec-properties-of-typedarray-instances)
    pub(crate) array_length: TypedArrayArrayLength,
}

impl TypedArrayHeapData {
    pub fn new(object_index: Option<OrdinaryObject>) -> Self {
        Self {
            object_index,
            viewed_array_buffer: ArrayBuffer::_def(),
            byte_length: Default::default(),
            byte_offset: Default::default(),
            array_length: Default::default(),
        }
    }
}

impl Default for TypedArrayHeapData {
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

impl HeapMarkAndSweep for TypedArrayHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
