// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{OrdinaryObject, Value},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct MapHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    // TODO: This isn't even close to a hashmap; HashMap won't allow inserting
    // Value as key; f32 isn't hashable. And our f64s are found on the Heap and
    // require fetching; What we actually should do is more like:
    // pub(crate) map: HashMap<ValueHash, u32>
    // pub(crate) key_values: ParallelVec<Option<Value<'gen>>, Option<Value<'gen>>>
    // ValueHash is created using a Value.hash(agent) function and connects to
    // an index; the index points to a key and value in parallel vector / Vec2.
    // Note that empty slots are deleted values in the ParallelVec.
    pub(crate) keys: Vec<Option<Value<'gen>>>,
    pub(crate) values: Vec<Option<Value<'gen>>>,
    // TODO: When an non-terminal (start or end) iterator exists for the Map,
    // the items in the map cannot be compacted.
    // pub(crate) observed: bool;
}

impl<'gen> HeapMarkAndSweep<'gen> for MapHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.object_index.mark_values(queues);
        self.keys.iter().for_each(|value| value.mark_values(queues));
        self.values
            .iter()
            .for_each(|value| value.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.keys
            .iter_mut()
            .for_each(|value| value.sweep_values(compactions));
        self.values
            .iter_mut()
            .for_each(|value| value.sweep_values(compactions));
    }
}
