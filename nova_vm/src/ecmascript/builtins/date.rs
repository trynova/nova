// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub(crate) use data::*;

use crate::{
    ecmascript::{
        Agent, InternalMethods, InternalSlots, OrdinaryObject, ProtoIntrinsics, object_handle,
    },
    engine::Bindable,
    heap::{
        ArenaAccess, ArenaAccessMut, BaseIndex, CompactionLists, CreateHeapData, Heap,
        HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access,
    },
};

/// ## [21.4 Date Objects](https://tc39.es/ecma262/#sec-date-objects)
///
/// Time measurement in ECMAScript is analogous to time measurement in POSIX, in
/// particular sharing definition in terms of the proleptic Gregorian calendar,
/// an _epoch_ of midnight at the beginning of 1 January 1970 UTC, and an
/// accounting of every day as comprising exactly 86,400 seconds (each of which
/// is 1000 milliseconds long).
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
