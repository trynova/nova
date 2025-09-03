// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
