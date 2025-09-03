// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ptr::NonNull;

use crate::{
    ecmascript::types::OrdinaryObject,
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct SharedArrayBufferHeapData<'a> {
    pub(super) backing_object: Option<OrdinaryObject<'a>>,
}

bindable_handle!(SharedArrayBufferHeapData);

/// # [Shared Data Block](https://tc39.es/ecma262/#sec-data-blocks)
///
/// The Data Block specification type is used to describe a distinct and
/// mutable sequence of byte-sized (8 bit) numeric values. A byte value
/// is an integer in the inclusive interval from 0 to 255. A Data Block
/// value is created with a fixed number of bytes that each have the
/// initial value 0.
///
/// The `ptr` points to a continuous buffer
/// of bytes, the length of which is determined by
/// the capacity. The pointer can be None if the
/// capacity of the buffer is zero.
#[derive(Debug, Clone)]
pub(crate) struct DataBlock {
    ptr: Option<NonNull<u8>>,
    byte_length: usize,
}

impl HeapMarkAndSweep for SharedArrayBufferHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { backing_object } = self;
        backing_object.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { backing_object } = self;
        backing_object.sweep_values(compactions);
    }
}
