// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::OrdinaryObject,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct FinalizationRegistryHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
}

impl<'gen> HeapMarkAndSweep<'gen> for FinalizationRegistryHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
