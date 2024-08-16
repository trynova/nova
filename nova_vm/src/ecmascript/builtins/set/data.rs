// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use hashbrown::HashTable;

use crate::{
    ecmascript::types::{OrdinaryObject, Value},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct SetHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) values: Vec<Option<Value>>,
    /// Low-level hash table pointing to value indexes.
    pub(crate) set_data: HashTable<u32>,
    // TODO: When an non-terminal (start or end) iterator exists for the Set,
    // the items in the map cannot be compacted.
    // pub(crate) observed: bool;
}

impl HeapMarkAndSweep for SetHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        self.values
            .iter()
            .for_each(|value| value.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.values
            .iter_mut()
            .for_each(|value| value.sweep_values(compactions));
    }
}
