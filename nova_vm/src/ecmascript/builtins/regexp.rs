use crate::{
    ecmascript::types::{IntoObject, IntoValue, Object, OrdinaryObject, Value},
    heap::{
        indexes::{BaseIndex, RegExpIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, Default)]
pub struct RegExpHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    // _regex: RegExp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RegExp(RegExpIndex);

impl RegExp {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<RegExp> for Value {
    fn from(value: RegExp) -> Self {
        Self::RegExp(value)
    }
}

impl From<RegExp> for Object {
    fn from(value: RegExp) -> Self {
        Self::RegExp(value)
    }
}

impl IntoValue for RegExp {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for RegExp {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl CreateHeapData<RegExpHeapData, RegExp> for Heap {
    fn create(&mut self, data: RegExpHeapData) -> RegExp {
        self.regexps.push(Some(data));
        RegExp(RegExpIndex::last(&self.regexps))
    }
}

impl HeapMarkAndSweep for RegExp {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.regexps.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 =
            RegExpIndex::from_u32(self_index - compactions.regexps.get_shift_for_index(self_index));
    }
}

impl HeapMarkAndSweep for RegExpHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
    }
}
