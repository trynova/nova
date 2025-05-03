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
        Scoped,
        context::{Bindable, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues, indexes::DateIndex,
    },
};

use self::data::DateHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Date<'a>(pub(crate) DateIndex<'a>);

impl Date<'_> {
    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Date<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

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
        Self(DateIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Date<'_> {
    type Of<'a> = Date<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<Date<'a>> for Value<'a> {
    fn from(value: Date<'a>) -> Self {
        Value::Date(value)
    }
}

impl<'a> From<Date<'a>> for Object<'a> {
    fn from(value: Date) -> Self {
        Object::Date(value.unbind())
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

impl Index<Date<'_>> for Agent {
    type Output = DateHeapData<'static>;

    fn index(&self, index: Date) -> &Self::Output {
        &self.heap.dates[index]
    }
}

impl IndexMut<Date<'_>> for Agent {
    fn index_mut(&mut self, index: Date) -> &mut Self::Output {
        &mut self.heap.dates[index]
    }
}

impl Index<Date<'_>> for Vec<Option<DateHeapData<'static>>> {
    type Output = DateHeapData<'static>;

    fn index(&self, index: Date) -> &Self::Output {
        self.get(index.get_index())
            .expect("Date out of bounds")
            .as_ref()
            .expect("Date slot empty")
    }
}

impl IndexMut<Date<'_>> for Vec<Option<DateHeapData<'static>>> {
    fn index_mut(&mut self, index: Date) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Date out of bounds")
            .as_mut()
            .expect("Date slot empty")
    }
}

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

impl<'a> CreateHeapData<DateHeapData<'a>, Date<'a>> for Heap {
    fn create(&mut self, data: DateHeapData<'a>) -> Date<'a> {
        self.dates.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<DateHeapData<'static>>>();
        Date(DateIndex::last(&self.dates))
    }
}
