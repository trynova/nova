// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) mod data;

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::DateIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

use self::data::DateHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Date(pub(crate) DateIndex);

impl Date {
    pub(crate) const fn _def() -> Self {
        Self(DateIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for Date {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl From<Date> for Value {
    fn from(value: Date) -> Self {
        Value::Date(value)
    }
}

impl IntoObject for Date {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Date> for Object {
    fn from(value: Date) -> Self {
        Object::Date(value)
    }
}

impl TryFrom<Value> for Date {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, ()> {
        match value {
            Value::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for Date {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, ()> {
        match value {
            Object::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl InternalSlots for Date {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Date;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }
}

impl InternalMethods for Date {}

impl Index<Date> for Agent {
    type Output = DateHeapData;

    fn index(&self, index: Date) -> &Self::Output {
        &self.heap.dates[index]
    }
}

impl IndexMut<Date> for Agent {
    fn index_mut(&mut self, index: Date) -> &mut Self::Output {
        &mut self.heap.dates[index]
    }
}

impl Index<Date> for Vec<Option<DateHeapData>> {
    type Output = DateHeapData;

    fn index(&self, index: Date) -> &Self::Output {
        self.get(index.get_index())
            .expect("Date out of bounds")
            .as_ref()
            .expect("Date slot empty")
    }
}

impl IndexMut<Date> for Vec<Option<DateHeapData>> {
    fn index_mut(&mut self, index: Date) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Date out of bounds")
            .as_mut()
            .expect("Date slot empty")
    }
}

impl HeapMarkAndSweep for Date {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.dates.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.dates.shift_index(&mut self.0);
    }
}

impl CreateHeapData<DateHeapData, Date> for Heap {
    fn create(&mut self, data: DateHeapData) -> Date {
        self.dates.push(Some(data));
        Date(DateIndex::last(&self.dates))
    }
}
