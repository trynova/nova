// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) mod data;

use core::ops::{Index, IndexMut};

use data::DateValue;

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::DateHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Date<'a>(BaseIndex<'a, DateHeapData<'static>>);

impl Date<'_> {
    /// ### get [[DateValue]]
    #[inline]
    pub(crate) fn date_value(self, agent: &Agent) -> DateValue {
        agent[self].date
    }

    /// ### set [[DateValue]]
    #[inline]
    pub(crate) fn set_date_value(self, agent: &mut Agent, date: DateValue) {
        agent[self].date = date;
    }

    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

bindable_handle!(Date);

impl<'a> From<Date<'a>> for Object<'a> {
    fn from(value: Date<'a>) -> Self {
        Object::Date(value)
    }
}

impl<'a> TryFrom<Value<'a>> for Date<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        match value {
            Value::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for Date<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, ()> {
        match value {
            Object::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for Date<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Date;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for Date<'a> {}

impl Rootable for Date<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Date(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Date(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for Date<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.dates.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.dates.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Date<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.dates.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<DateHeapData<'a>, Date<'a>> for Heap {
    fn create(&mut self, data: DateHeapData<'a>) -> Date<'a> {
        self.dates.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<DateHeapData<'static>>();
        Date(BaseIndex::last(&self.dates))
    }
}
