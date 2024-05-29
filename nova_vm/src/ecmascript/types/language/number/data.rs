use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone, Copy)]
pub struct NumberHeapData {
    pub(crate) data: f64,
}

impl From<f64> for NumberHeapData {
    #[inline(always)]
    fn from(data: f64) -> Self {
        Self { data }
    }
}

impl From<NumberHeapData> for f64 {
    fn from(value: NumberHeapData) -> Self {
        value.data
    }
}

impl HeapMarkAndSweep for NumberHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues) {}

    fn sweep_values(&mut self, _compactions: &CompactionLists) {}
}
