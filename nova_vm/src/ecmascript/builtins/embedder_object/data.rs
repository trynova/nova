// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::OrdinaryObject,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone)]
pub struct EmbedderObjectHeapData<'a> {
    backing_object: Option<OrdinaryObject<'a>>,
}

impl HeapMarkAndSweep for EmbedderObjectHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { backing_object } = self;
        backing_object.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { backing_object } = self;
        backing_object.sweep_values(compactions);
    }
}
