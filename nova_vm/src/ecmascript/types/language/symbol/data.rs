// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::String,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData<'gen> {
    pub(crate) descriptor: Option<String<'gen>>,
}

impl<'gen> HeapMarkAndSweep<'gen> for SymbolHeapData<'_> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.descriptor.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.descriptor.sweep_values(compactions);
    }
}
