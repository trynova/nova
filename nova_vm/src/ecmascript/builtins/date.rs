// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, OrdinaryObject, object_handle},
    },
    engine::context::Bindable,
    heap::{
        ArenaAccess, ArenaAccessMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, WorkQueues, arena_vec_access, indexes::BaseIndex,
    },
};

pub(crate) use self::data::DateHeapData;
pub(super) use data::DateValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Date<'a>(BaseIndex<'a, DateHeapData<'static>>);
object_handle!(Date);
arena_vec_access!(
    Date,
    'a,
    DateHeapData,
    dates
);

impl Date<'_> {
    /// ### get [[DateValue]]
    #[inline]
    pub(crate) fn date_value(self, agent: &Agent) -> DateValue {
        self.get(agent).date
    }

    /// ### set [[DateValue]]
    #[inline]
    pub(crate) fn set_date_value(self, agent: &mut Agent, date: DateValue) {
        self.get_mut(agent).date = date;
    }
}

impl<'a> InternalSlots<'a> for Date<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Date;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for Date<'a> {}

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
