// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};
use num_bigint::BigInt;

#[derive(Debug, Clone)]
pub struct BigIntHeapData {
    pub(crate) data: BigInt,
}

impl HeapMarkAndSweep for BigIntHeapData {
    #[inline(always)]
    fn mark_values(&self, _queues: &mut WorkQueues) {
        let Self { data: _ } = self;
    }

    #[inline(always)]
    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        let Self { data: _ } = self;
    }
}
