// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod plain_time_constructor;
mod plain_time_prototype;

pub(crate) use data::*;
pub(crate) use plain_time_constructor::*;
pub(crate) use plain_time_prototype::*;

use temporal_rs::options::{Unit, UnitGroup};

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


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]

pub struct TemporalPlainTime<'a>(BaseIndex<'a, PlainTimeRecord<'static>>);
object_handle!(TemporalPlainTime, plain_time);
arena_vec_access!(
    TemporalPlainTime,
    'a,
    PlainTimeRecord,
    plain_time
);

impl TemporalPlainTime<'_> {
    pub(crate) fn inner_plain_time(self, agent: &Agent) -> &temporal_rs::PlainTime {
        &self.unbind().get(agent).plain_time
    }
}

impl<'a> InternalSlots<'a> for TemporalPlainTime<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::TemporalPlainTime;
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


impl<'a> InternalMethods<'a> for TemporalPlainTime<'a> {}

impl HeapMarkAndSweep for TemporalPlainTime<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.plain_times.push(*self);
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.plain_times.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for TemporalPlainTime<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.plain_times.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<PlainTimeRecord<'a>, TemporalPlainTime<'a>> for Heap {
    fn create(&mut self, data: PlainTimeRecord<'a>) -> TemporalPlainTime<'a> {
        self.plain_times.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<PlainTimeRecord<'static>>();
        TemporalPlainTime(BaseIndex::last(&self.plain_times))
    }
}