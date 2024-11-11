// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::String,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData {
    pub(crate) descriptor: Option<String>,
}

impl HeapMarkAndSweep for SymbolHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { descriptor } = self;
        descriptor.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { descriptor } = self;
        descriptor.sweep_values(compactions);
    }
}
