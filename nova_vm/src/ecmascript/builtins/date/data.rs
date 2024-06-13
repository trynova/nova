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
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
