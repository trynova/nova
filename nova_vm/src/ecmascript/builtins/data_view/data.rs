// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            ArrayBuffer,
            array_buffer::{ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
            shared_array_buffer::SharedArrayBuffer,
        },
        types::OrdinaryObject,
    },
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

pub struct DataViewRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    // TODO: Add a helper function for a u32::MAX value which signifies an a under-construction value:
    // See https://github.com/trynova/nova/pull/447#discussion_r1806247107 for reference.
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) viewed_array_buffer: ArrayBuffer<'a>,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) byte_length: ViewedArrayBufferByteLength,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) byte_offset: ViewedArrayBufferByteOffset,
}
bindable_handle!(DataViewRecord);

impl Default for DataViewRecord<'_> {
    fn default() -> Self {
        Self {
            object_index: None,
            viewed_array_buffer: ArrayBuffer::_def(),
            byte_length: ViewedArrayBufferByteLength::default(),
            byte_offset: ViewedArrayBufferByteOffset::default(),
        }
    }
}

#[cfg(feature = "shared-array-buffer")]
pub struct SharedDataViewRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    // TODO: Add a helper function for a u32::MAX value which signifies an a under-construction value:
    // See https://github.com/trynova/nova/pull/447#discussion_r1806247107 for reference.
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) viewed_array_buffer: SharedArrayBuffer<'a>,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) byte_length: ViewedArrayBufferByteLength,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) byte_offset: ViewedArrayBufferByteOffset,
}
#[cfg(feature = "shared-array-buffer")]
bindable_handle!(SharedDataViewRecord);

#[cfg(feature = "shared-array-buffer")]
impl Default for SharedDataViewRecord<'_> {
    fn default() -> Self {
        Self {
            object_index: None,
            viewed_array_buffer: SharedArrayBuffer::_DEF,
            byte_length: ViewedArrayBufferByteLength::default(),
            byte_offset: ViewedArrayBufferByteOffset::default(),
        }
    }
}

impl HeapMarkAndSweep for DataViewRecord<'static> {
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

#[cfg(feature = "shared-array-buffer")]
impl HeapMarkAndSweep for SharedDataViewRecord<'static> {
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
