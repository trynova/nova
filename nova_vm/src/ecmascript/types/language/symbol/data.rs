use crate::{
    ecmascript::types::String,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData {
    pub(crate) descriptor: Option<String>,
}

impl HeapMarkAndSweep for SymbolHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.descriptor.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.descriptor.sweep_values(compactions);
    }
}
