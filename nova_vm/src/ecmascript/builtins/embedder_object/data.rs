// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone)]
pub struct EmbedderObjectHeapData {}

impl HeapMarkAndSweep for EmbedderObjectHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues) {
        let Self {} = self;
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        let Self {} = self;
    }
}
