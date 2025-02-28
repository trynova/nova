// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) mod data;

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{
        context::NoGcScope,
        rootable::{HeapRootData, HeapRootRef, Rootable},
        Scoped,
    },
    heap::{
        indexes::DateIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

use self::data::DateHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Date<'a>(pub(crate) DateIndex<'a>);

impl Date<'_> {
    /// Unbind this Date from its current lifetime. This is necessary to use
    /// the Date as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> Date<'static> {
        unsafe { core::mem::transmute::<Self, Date<'static>>(self) }
    }

    // Bind this Date to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your Dates cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let date = date.bind(&gc);
    // ```
    // to make sure that the unbound Date cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> Date<'gc> {
        unsafe { core::mem::transmute::<Date, Date<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Date<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        Self(DateIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'a> IntoValue<'a> for Date<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> From<Date<'a>> for Value<'a> {
    fn from(value: Date<'a>) -> Self {
        Value::Date(value)
    }
}

impl<'a> IntoObject<'a> for Date<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
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
    type Output = DateHeapData;

    fn index(&self, index: Date) -> &Self::Output {
        &self.heap.dates[index]
    }
}

impl IndexMut<Date<'_>> for Agent {
    fn index_mut(&mut self, index: Date) -> &mut Self::Output {
        &mut self.heap.dates[index]
    }
}

impl Index<Date<'_>> for Vec<Option<DateHeapData>> {
    type Output = DateHeapData;

    fn index(&self, index: Date) -> &Self::Output {
        self.get(index.get_index())
            .expect("Date out of bounds")
            .as_ref()
            .expect("Date slot empty")
    }
}

impl IndexMut<Date<'_>> for Vec<Option<DateHeapData>> {
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

impl CreateHeapData<DateHeapData, Date<'static>> for Heap {
    fn create(&mut self, data: DateHeapData) -> Date<'static> {
        self.dates.push(Some(data));
        Date(DateIndex::last(&self.dates))
    }
}
