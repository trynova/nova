// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{OrdinaryObject, SharedDataBlock},
    engine::context::{NoGcScope, bindable_handle},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Clone, Default)]
pub(crate) struct SharedArrayBufferRecord<'a> {
    pub(super) backing_object: Option<OrdinaryObject<'a>>,
    pub(super) data_block: SharedDataBlock,
}
bindable_handle!(SharedArrayBufferRecord);

impl<'a> SharedArrayBufferRecord<'a> {
    pub(crate) fn new(block: SharedDataBlock, _: NoGcScope<'a, '_>) -> Self {
        Self {
            backing_object: None,
            data_block: block,
        }
    }
}

impl HeapMarkAndSweep for SharedArrayBufferRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            backing_object,
            data_block: _,
        } = self;
        backing_object.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            backing_object,
            data_block: _,
        } = self;
        backing_object.sweep_values(compactions);
    }
}
