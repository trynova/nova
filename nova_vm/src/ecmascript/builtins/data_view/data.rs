// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{builtins::ArrayBuffer, types::OrdinaryObject},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct DataViewHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    /// ### [\[\[ViewedArrayBuffer\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) viewed_array_buffer: Option<ArrayBuffer>,
    /// ### [\[\[ByteLength\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    ///
    /// NOTE: A `None` value indicates that the byte length is `AUTO`.
    pub(crate) byte_length: Option<usize>,
    /// ### [\[\[ByteOffset\]\]](https://tc39.es/ecma262/#sec-properties-of-dataview-instances)
    pub(crate) byte_offset: usize,
}

impl HeapMarkAndSweep for DataViewHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        self.viewed_array_buffer.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.viewed_array_buffer.sweep_values(compactions);
    }
}
