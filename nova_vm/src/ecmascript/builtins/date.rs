// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub(crate) mod data;

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData, Value,
        },
    },
    heap::{
        indexes::DateIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

use self::data::DateHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Date<'gen>(pub(crate) DateIndex<'gen>);

impl<'gen> Date<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(DateIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> IntoValue<'gen> for Date<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> From<Date<'gen>> for Value<'gen> {
    fn from(value: Date<'gen>) -> Self {
        Value::Date(value)
    }
}

impl<'gen> IntoObject<'gen> for Date<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<Date<'gen>> for Object<'gen> {
    fn from(value: Date<'gen>) -> Self {
        Object::Date(value)
    }
}

impl<'gen> TryFrom<Value<'gen>> for Date<'gen> {
    type Error = ();

    fn try_from(value: Value<'gen>) -> Result<Self, ()> {
        match value {
            Value::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'gen> TryFrom<Object<'gen>> for Date<'gen> {
    type Error = ();

    fn try_from(value: Object<'gen>) -> Result<Self, ()> {
        match value {
            Object::Date(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'gen> InternalSlots<'gen> for Date<'gen> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Date;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent<'gen>) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl<'gen> InternalMethods<'gen> for Date<'gen> {}

impl<'gen> Index<Date<'gen>> for Agent<'gen> {
    type Output = DateHeapData<'gen>;

    fn index(&self, index: Date) -> &Self::Output {
        &self.heap.dates[index]
    }
}

impl<'gen> IndexMut<Date<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: Date) -> &mut Self::Output {
        &mut self.heap.dates[index]
    }
}

impl<'gen> Index<Date<'gen>> for Vec<Option<DateHeapData<'gen>>> {
    type Output = DateHeapData<'gen>;

    fn index(&self, index: Date<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Date out of bounds")
            .as_ref()
            .expect("Date slot empty")
    }
}

impl<'gen> IndexMut<Date<'gen>> for Vec<Option<DateHeapData<'gen>>> {
    fn index_mut(&mut self, index: Date<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Date out of bounds")
            .as_mut()
            .expect("Date slot empty")
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for Date<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.dates.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.dates.shift_index(&mut self.0);
    }
}

impl<'gen> CreateHeapData<DateHeapData<'gen>, Date<'gen>> for Heap<'gen> {
    fn create(&mut self, data: DateHeapData<'gen>) -> Date<'gen> {
        self.dates.push(Some(data));
        Date(DateIndex::last(&self.dates))
    }
}
