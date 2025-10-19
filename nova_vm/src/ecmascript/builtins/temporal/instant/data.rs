
use crate::{ecmascript::types::{OrdinaryObject,bigint::BigInt}, engine::context::bindable_handle, heap::HeapMarkAndSweep};

#[derive(Debug, Clone, Copy)]
pub struct InstantHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) instant: BigInt<'a>,
}

impl InstantHeapData<'_> {
    pub fn default() -> Self {
        Self {
            object_index: None,
            instant: BigInt::zero(),
        }
    }
}

bindable_handle!(InstantHeapData);

impl HeapMarkAndSweep for InstantHeapData<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        todo!()
    }
    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        todo!()
    }
}
