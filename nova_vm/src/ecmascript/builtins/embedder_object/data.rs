use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone)]
pub struct EmbedderObjectHeapData {}

impl HeapMarkAndSweep for EmbedderObjectHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues) {}

    fn sweep_values(&mut self, _compactions: &CompactionLists) {}
}
