use crate::{
    ecmascript::types::{OrdinaryObject, Value},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Default)]
pub struct SetHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    // TODO: This isn't even close to a hashmap; HashSet won't allow inserting
    // Value as key; f32 isn't hashable. And our f64s are found on the Heap and
    // require fetching; What we actually should do is more like:
    // pub(crate) map: HashSet<ValueHash, u32>
    // pub(crate) values: Vec<Option<Value>>
    // ValueHash is created using a Value.hash(agent) function and connects to
    // an index; the index points to a value in Vec.
    // Note that empty slots are deleted values in the Vec.
    pub(crate) set: Vec<Value>,
    // TODO: When an non-terminal (start or end) iterator exists for the Set,
    // the items in the map cannot be compacted.
    // pub(crate) observed: bool;
}

impl HeapMarkAndSweep for SetHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
        self.set.iter().for_each(|value| value.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.set
            .iter_mut()
            .for_each(|value| value.sweep_values(compactions));
    }
}
