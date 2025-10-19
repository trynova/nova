use crate::{
    ecmascript::types::OrdinaryObject,
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct InstantRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) instant: temporal_rs::Instant,
}

impl InstantRecord<'_> {
    pub fn default() -> Self {
        Self {
            object_index: None,
            instant: temporal_rs::Instant::try_new(0).unwrap(),
        }
    }
}

bindable_handle!(InstantRecord);

impl HeapMarkAndSweep for InstantRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            instant: _,
        } = self;

        object_index.mark_values(queues);
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            instant: _,
        } = self;

        object_index.sweep_values(compactions);
    }
}
