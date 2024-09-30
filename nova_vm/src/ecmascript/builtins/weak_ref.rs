// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, WeakRefIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::WeakRefHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct WeakRef(pub(crate) WeakRefIndex);

impl WeakRef {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<WeakRef> for WeakRefIndex {
    fn from(val: WeakRef) -> Self {
        val.0
    }
}

impl From<WeakRefIndex> for WeakRef {
    fn from(value: WeakRefIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for WeakRef {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for WeakRef {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<WeakRef> for Value {
    fn from(val: WeakRef) -> Self {
        Value::WeakRef(val)
    }
}

impl From<WeakRef> for Object {
    fn from(val: WeakRef) -> Self {
        Object::WeakRef(val)
    }
}

impl InternalSlots for WeakRef {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::WeakRef;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl InternalMethods for WeakRef {}

impl Index<WeakRef> for Agent {
    type Output = WeakRefHeapData;

    fn index(&self, index: WeakRef) -> &Self::Output {
        &self.heap.weak_refs[index]
    }
}

impl IndexMut<WeakRef> for Agent {
    fn index_mut(&mut self, index: WeakRef) -> &mut Self::Output {
        &mut self.heap.weak_refs[index]
    }
}

impl Index<WeakRef> for Vec<Option<WeakRefHeapData>> {
    type Output = WeakRefHeapData;

    fn index(&self, index: WeakRef) -> &Self::Output {
        self.get(index.get_index())
            .expect("WeakRef out of bounds")
            .as_ref()
            .expect("WeakRef slot empty")
    }
}

impl IndexMut<WeakRef> for Vec<Option<WeakRefHeapData>> {
    fn index_mut(&mut self, index: WeakRef) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("WeakRef out of bounds")
            .as_mut()
            .expect("WeakRef slot empty")
    }
}

impl CreateHeapData<WeakRefHeapData, WeakRef> for Heap {
    fn create(&mut self, data: WeakRefHeapData) -> WeakRef {
        self.weak_refs.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakRef(WeakRefIndex::last(&self.weak_refs))
    }
}

impl HeapMarkAndSweep for WeakRef {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.weak_refs.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.weak_refs.shift_index(&mut self.0);
    }
}
