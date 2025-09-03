// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            ArrayBuffer,
            array_buffer::{ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset},
        },
        types::OrdinaryObject,
    },
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone)]
pub struct DataViewHeapData<'a> {
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

impl Default for DataViewHeapData<'_> {
    fn default() -> Self {
        Self {
            object_index: None,
            viewed_array_buffer: ArrayBuffer::_def(),
            byte_length: ViewedArrayBufferByteLength::default(),
            byte_offset: ViewedArrayBufferByteOffset::default(),
        }
    }
}

bindable_handle!(DataViewHeapData);

impl HeapMarkAndSweep for DataViewHeapData<'static> {
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
