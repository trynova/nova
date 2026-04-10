// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod plain_time_constructor;
mod plain_time_prototype;

pub(crate) use data::*;
pub(crate) use plain_time_constructor::*;
pub(crate) use plain_time_prototype::*;

use crate::{
    ecmascript::{
        Agent, ExceptionType, Function, InternalMethods, InternalSlots, JsResult, OrdinaryObject,
        ProtoIntrinsics, Value, object_handle, ordinary_populate_from_constructor,
        temporal_err_to_js_err, to_temporal_duration,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable},
    heap::{
        ArenaAccess, ArenaAccessMut, BaseIndex, CompactionLists, CreateHeapData, Heap,
        HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access,
    },
};

/// # [4 Temporal.PlainTime Objects](https://tc39.es/proposal-temporal/#sec-temporal-plaintime-objects)
///
/// A Temporal.PlainTime object is an Object that contains integers
/// corresponding to a particular hour, minute, second, millisecond,
/// microsecond, and nanosecond.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TemporalPlainTime<'a>(BaseIndex<'a, PlainTimeRecord<'static>>);
object_handle!(TemporalPlainTime, PlainTime);
arena_vec_access!(
    TemporalPlainTime,
    'a,
    PlainTimeRecord,
    plain_times
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

#[inline(always)]
fn require_internal_slot_temporal_plain_time<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, TemporalPlainTime<'a>> {
    match value {
        Value::PlainTime(plain_time) => Ok(plain_time.bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Object is not a Temporal PlainTime",
            gc,
        )),
    }
}

/// ### [4.5.11 CreateTemporalTime](https://tc39.es/proposal-temporal/#sec-temporal-createtemporaltime)
pub(crate) fn create_temporal_plain_time<'gc>(
    agent: &mut Agent,
    plain_time: temporal_rs::PlainTime,
    new_target: Option<Function>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, TemporalPlainTime<'gc>> {
    // 1. If newTarget is not present, set newTarget to %Temporal.PlainTime%.
    let new_target = new_target.unwrap_or_else(|| {
        agent
            .current_realm_record()
            .intrinsics()
            .temporal_plain_time()
            .into()
    });
    // 2. Let object be ? OrdinaryCreateFromConstructor(newTarget, "%Temporal.PlainTime.prototype%", « [[InitializedTemporalTime]], [[Time]] »).
    // 3. Set object.[[Time]] to time.
    // 4. Return object.
    let object = agent.heap.create(PlainTimeRecord {
        object_index: None,
        plain_time,
    });
    Ok(
        TemporalPlainTime::try_from(ordinary_populate_from_constructor(
            agent,
            object.unbind().into(),
            new_target,
            ProtoIntrinsics::TemporalPlainTime,
            gc,
        )?)
        .unwrap(),
    )
}

/// [4.5.18 AddDurationToTime ( operation, temporalTime, temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal-adddurationtotime)
///
/// The abstract operation AddDurationToTime takes arguments operation
/// (either add or subtract), temporalTime (a Temporal.PlainTime), and
/// temporalDurationLike (an ECMAScript language value) and returns either
/// a normal completion containing a Temporal.PlainTime or a throw completion.
/// It adds/subtracts temporalDurationLike to/from temporalTime, returning a
/// point in time that is in the future/past relative to temporalTime.
/// It performs the following steps when called:
fn add_duration_to_time<'gc, const IS_ADD: bool>(
    agent: &mut Agent,
    plan_time: TemporalPlainTime,
    duration: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, TemporalPlainTime<'gc>> {
    let duration = duration.bind(gc.nogc());
    let mut plain_time = plan_time.bind(gc.nogc());

    // 1. Let duration be ? ToTemporalDuration(temporalDurationLike).
    let duration = if let Value::Duration(duration) = duration {
        duration.get(agent).duration
    } else {
        let scoped_instant = plain_time.scope(agent, gc.nogc());
        let res = to_temporal_duration(agent, duration.unbind(), gc.reborrow()).unbind()?;
        // SAFETY: not shared
        unsafe {
            plain_time = scoped_instant.take(agent);
        }
        res
    };

    // If operation is subtract, set duration to CreateNegatedTemporalDuration(duration).
    // 3. Let internalDuration be ToInternalDurationRecord(duration).
    // 4. Let result be AddTime(temporalTime.[[Time]], internalDuration.[[Time]]).
    let ns_result = if IS_ADD {
        temporal_rs::PlainTime::add(plain_time.inner_plain_time(agent), &duration)
            .map_err(|err| temporal_err_to_js_err(agent, err, gc.nogc()))
            .unbind()?
    } else {
        temporal_rs::PlainTime::subtract(plain_time.inner_plain_time(agent), &duration)
            .map_err(|err| temporal_err_to_js_err(agent, err, gc.nogc()))
            .unbind()?
    };

    // 5. Return ! CreateTemporalTime(result).
    Ok(create_temporal_plain_time(agent, ns_result, None, gc).unwrap())
}
