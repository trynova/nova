// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone, Copy)]
pub struct NumberHeapData {
    pub(crate) data: f64,
}

impl From<f64> for NumberHeapData {
    #[inline(always)]
    fn from(data: f64) -> Self {
        Self { data }
    }
}

impl From<NumberHeapData> for f64 {
    fn from(value: NumberHeapData) -> Self {
        value.data
    }
}

impl HeapMarkAndSweep for NumberHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues) {
        let Self { data: _ } = self;
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        let Self { data: _ } = self;
    }
}
