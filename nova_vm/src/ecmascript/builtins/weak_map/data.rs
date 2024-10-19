// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{OrdinaryObject, Value},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct WeakMapHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    // TODO: This isn't even close to a hashmap; HashMap won't allow inserting
    // Value as key; f32 isn't hashable. And our f64s are found on the Heap and
    // require fetching; What we actually should do is more like:
    // pub(crate) map: HashMap<ValueHash, u32>
    // pub(crate) key_values: ParallelVec<Option<Value>, Option<Value>>
    // ValueHash is created using a Value.hash(agent) function and connects to
    // an index; the index points to a key and value in parallel vector / Vec2.
    // Note that empty slots are deleted values in the ParallelVec.
    pub(crate) keys: Vec<Value>,
    pub(crate) values: Vec<Value>,
    // TODO: When an non-terminal (start or end) iterator exists for the Map,
    // the items in the map cannot be compacted.
    // pub(crate) observed: bool;
}

impl HeapMarkAndSweep for WeakMapHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            keys,
            values,
        } = self;
        object_index.mark_values(queues);
        for ele in keys {
            ele.mark_values(queues);
        }
        for ele in values {
            ele.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            keys,
            values,
        } = self;
        object_index.sweep_values(compactions);
        for ele in keys {
            ele.sweep_values(compactions);
        }
        for ele in values {
            ele.sweep_values(compactions);
        }
    }
}
