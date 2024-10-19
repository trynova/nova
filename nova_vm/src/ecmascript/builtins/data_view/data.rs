// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{builtins::ArrayBuffer, types::OrdinaryObject},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct DataViewByteLength(pub u32);

impl DataViewByteLength {
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

impl Default for DataViewByteLength {
    fn default() -> Self {
        Self::auto()
    }
}

impl From<Option<usize>> for DataViewByteLength {
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
pub(crate) struct DataViewByteOffset(pub u32);

impl DataViewByteOffset {
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

impl Default for DataViewByteOffset {
    fn default() -> Self {
        Self::value(0)
    }
}

impl From<usize> for DataViewByteOffset {
    fn from(value: usize) -> Self {
        if value >= Self::heap().0 as usize {
            Self::heap()
        } else {
            Self::value(value as u32)
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataViewHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    // TODO: Add a helper function for a u32::MAX value which signifies an a under-construction value:
    // See https://github.com/trynova/nova/pull/447#discussion_r1806247107 for reference.
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) viewed_array_buffer: ArrayBuffer,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) byte_length: DataViewByteLength,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) byte_offset: DataViewByteOffset,
}

impl Default for DataViewHeapData {
    fn default() -> Self {
        Self {
            object_index: None,
            viewed_array_buffer: ArrayBuffer::_def(),
            byte_length: DataViewByteLength::default(),
            byte_offset: DataViewByteOffset::default(),
        }
    }
}

impl HeapMarkAndSweep for DataViewHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            viewed_array_buffer,
            byte_length: _,
            byte_offset: _,
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
        } = self;
        object_index.sweep_values(compactions);
        viewed_array_buffer.sweep_values(compactions);
    }
}
