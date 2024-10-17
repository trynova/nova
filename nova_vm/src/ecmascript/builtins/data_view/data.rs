// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::OrdinaryObject,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct DataViewHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}

impl HeapMarkAndSweep for DataViewHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { object_index } = self;
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { object_index } = self;
        object_index.sweep_values(compactions);
    }
}
