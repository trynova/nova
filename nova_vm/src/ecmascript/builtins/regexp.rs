pub(crate) mod abstract_operations;
pub(crate) mod data;

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::Agent,
        types::{IntoObject, IntoValue, Object, Value},
    },
    heap::{
        indexes::{BaseIndex, RegExpIndex},
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};
pub(crate) use abstract_operations::*;
pub(crate) use data::RegExpHeapData;

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

impl TryFrom<Object> for RegExp {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
    }
}

impl TryFrom<Value> for RegExp {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::RegExp(regexp) => Ok(regexp),
            _ => Err(()),
        }
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

impl Index<RegExp> for Agent {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExp) -> &Self::Output {
        &self.heap.regexps[index]
    }
}

impl IndexMut<RegExp> for Agent {
    fn index_mut(&mut self, index: RegExp) -> &mut Self::Output {
        &mut self.heap.regexps[index]
    }
}

impl Index<RegExp> for Vec<Option<RegExpHeapData>> {
    type Output = RegExpHeapData;

    fn index(&self, index: RegExp) -> &Self::Output {
        self.get(index.get_index())
            .expect("RegExp out of bounds")
            .as_ref()
            .expect("RegExp slot empty")
    }
}

impl IndexMut<RegExp> for Vec<Option<RegExpHeapData>> {
    fn index_mut(&mut self, index: RegExp) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("RegExp out of bounds")
            .as_mut()
            .expect("RegExp slot empty")
    }
}

impl CreateHeapData<RegExpHeapData, RegExp> for Heap {
    fn create(&mut self, data: RegExpHeapData) -> RegExp {
        self.regexps.push(Some(data));
        RegExp(RegExpIndex::last(&self.regexps))
    }
}
