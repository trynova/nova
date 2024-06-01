use crate::{
    ecmascript::types::OrdinaryObject,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone)]
pub struct ProxyHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
}

impl HeapMarkAndSweep for ProxyHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
