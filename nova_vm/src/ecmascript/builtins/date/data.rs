// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::OrdinaryObject,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
pub struct DateHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) date: Option<SystemTime>,
}

impl DateHeapData {
    pub(crate) fn new_invalid() -> Self {
        Self {
            object_index: None,
            date: None,
        }
    }
}

impl HeapMarkAndSweep for DateHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            date: _,
        } = self;
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            date: _,
        } = self;
        object_index.sweep_values(compactions);
    }
}
