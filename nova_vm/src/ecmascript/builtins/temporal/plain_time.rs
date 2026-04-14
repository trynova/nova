// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod plain_time_constructor;
mod plain_time_prototype;

pub(crate) use data::*;
pub(crate) use plain_time_constructor::*;
pub(crate) use plain_time_prototype::*;
use sonic_rs::{Object, value::object::IterMut};
use temporal_rs::{
    duration,
    options::{Unit, UnitGroup},
};

use crate::{
    ecmascript::{
        Agent, DurationRecord, ExceptionType, Function, InternalMethods, InternalSlots, JsResult,
        Object, OrdinaryObject, ProtoIntrinsics, String, TemporalDuration, Value,
        get_difference_settings, get_options_object, object_handle,
        ordinary_populate_from_constructor, temporal_err_to_js_err,
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

/// ### [4.5.6 ToTemporalTime ( item [ , options ] )](https://tc39.es/proposal-temporal/#sec-temporal-totemporaltime)
///
/// The abstract operation ToTemporalTime takes argument item (an ECMAScript language value) and optional argument
/// options (an ECMAScript language value) and returns either a normal completion containing a Temporal.PlainTime
/// or a throw Completion. Converts item to a new Temporal.PlainTime instance if possible, and throws otherwise.
pub(crate) fn to_temporal_time<'gc>(
    agent: &mut Agent,
    item: Value,
    options: Option<Value>,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::PlainTime> {
    let item = item.bind(gc.nogc());

    // 1. If options is not present, set options to undefined.
    let options = options.unwrap_or(Value::Undefined);

    // 2. If item is an Object, then
    if let Ok(item) = Object::try_from(item) {
        // a. If item has an [[InitializedTemporalTime]] internal slot, then
        if let Ok(time) = TemporalPlainTime::try_from(item) {
            // i. Let resolvedOptions be ? GetOptionsObject(options).
            let resolved_options = get_options_object(agent, options, gc.nogc()).unbind()?;
            // ii. Perform ? GetTemporalOverflowOption(resolvedOptions).
            get_temporal_overflow_option(agent, resolved_options, gc.reborrow()).unbind()?;
            // iii. Return ! CreateTemporalTime(item.[[Time]]).
            return Ok(*time.inner_plain_time(agent));
        }

        // b. If item has an [[InitializedTemporalDateTime]] internal slot, then
        // i. Let resolvedOptions be ? GetOptionsObject(options).
        // ii. Perform ? GetTemporalOverflowOption(resolvedOptions).
        // iii. Return ! CreateTemporalTime(item.[[ISODateTime]].[[Time]]).

        // c. If item has an [[InitializedTemporalZonedDateTime]] internal slot, then
        // i. Let isoDateTime be GetISODateTimeFor(item.[[TimeZone]], item.[[EpochNanoseconds]]).
        // ii. Let resolvedOptions be ? GetOptionsObject(options).
        // iii. Perform ? GetTemporalOverflowOption(resolvedOptions).
        // iv. Return ! CreateTemporalTime(isoDateTime.[[Time]]).

        // d. Let result be ? ToTemporalTimeRecord(item).
        let result = to_temporal_time_record(agent, item.unbind(), gc.reborrow()).unbind()?;
        // e. Let resolvedOptions be ? GetOptionsObject(options).
        let resolved_options = get_options_object(agent, options, gc.nogc()).unbind()?;
        // f. Let overflow be ? GetTemporalOverflowOption(resolvedOptions).
        let overflow =
            get_temporal_overflow_option(agent, resolved_options, gc.reborrow()).unbind()?;
        // g. Set result to ? RegulateTime(result.[[Hour]], result.[[Minute]], result.[[Second]], result.[[Millisecond]], result.[[Microsecond]], result.[[Nanosecond]], overflow).
        return result
            .regulate(overflow)
            .map_err(|err| temporal_err_to_js_err(agent, err, gc.into_nogc()));
    }

    // 3. Else,
    // a. If item is not a String, throw a TypeError exception.
    let Ok(item) = String::try_from(item) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "item is not a String",
            gc.into_nogc(),
        ));
    };

    // b. Let parseResult be ? ParseISODateTime(item, « TemporalTimeString »).
    // c. Assert: parseResult.[[Time]] is not start-of-day.
    // d. Set result to parseResult.[[Time]].
    // e. NOTE: A successful parse using TemporalTimeString guarantees absence of ambiguity with respect to any ISO 8601 date-only, year-month, or month-day representation.
    let result = temporal_rs::PlainTime::from_utf8(item.as_bytes(agent))
        .map_err(|err| temporal_err_to_js_err(agent, err, gc.into_nogc()))?;
    // f. Let resolvedOptions be ? GetOptionsObject(options).
    let resolved_options = get_options_object(agent, options, gc.reborrow()).unbind()?;
    // g. Perform ? GetTemporalOverflowOption(resolvedOptions).
    get_temporal_overflow_option(agent, resolved_options, gc.reborrow()).unbind()?;

    // 4. Return ! CreateTemporalTime(result).
    Ok(result)
}

/// ### [4.5.17 DifferenceTemporalPlainTime ( operation, temporalTime, other, options )](https://tc39.es/proposal-temporal/#sec-temporal-differencetemporalplaintime)
/// The abstract operation DifferenceTemporalPlainTime takes arguments
/// operation (either since or until), temporalTime (a Temporal.PlainTime),
/// other (an ECMAScript language value), and options
/// (an ECMAScript language value) and returns either
/// a normal completion containing a Temporal.Duration or a
/// throw completion. It computes the difference between the
/// two times represented by temporalTime and other, optionally
/// rounds it, and returns it as a Temporal.Duration object.
fn difference_temporal_plain_time<'gc, const IS_UNTIL: bool>(
    agent: &mut Agent,
    plain_time: TemporalPlainTime,
    other: Value,
    options: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, TemporalDuration<'gc>> {
    let plain_time = plain_time.scope(agent, gc.nogc());
    let other = other.bind(gc.nogc());
    let options = options.scope(agent, gc.nogc());
    // 1. Set other to ? ToTemporalTime(other).
    let other = to_temporal_time(agent, other.unbind(), options, gc.reborrow());
    // 2. Let resolvedOptions be ? GetOptionsObject(options).
    let resolved_option = get_options_object(agent, options.get(agent), gc.nogc())
        .unbind()?
        .bind(gc.nogc());
    // 3. Let settings be ? GetDifferenceSettings(operation, resolvedOptions,
    // time, « », nanosecond, hour).
    // 4. Let timeDuration be
    // DifferenceTime(temporalTime.[[Time]], other.[[Time]]).
    // 5. Set timeDuration to ! RoundTimeDuration(timeDuration,
    // settings.[[RoundingIncrement]], settings.[[SmallestUnit]], settings.[[RoundingMode]]).
    // 6. Let duration be
    // CombineDateAndTimeDuration(ZeroDateDuration(), timeDuration).
    // 7. Let result be !
    // TemporalDurationFromInternal(duration, settings.[[LargestUnit]]).
    // 8. If operation is since, set result to
    // CreateNegatedTemporalDuration(result).
    let duration = if IS_UNTIL {
        const UNTIL: bool = true;
        let settings = get_difference_settings::<UNTIL>(
            agent,
            resolved_option.unbind(),
            UnitGroup::Time,
            &[],
            Unit::Nanosecond,
            Unit::Hour,
            gc.reborrow(),
        )
        .unbind()?;
        temporal_rs::PlainTime::until(
            plain_time.get(agent).inner_plain_time(agent),
            &other.unbind()?,
            settings,
        )
    } else {
        const SINCE: bool = false;
        let settings = get_difference_settings::<SINCE>(
            agent,
            resolved_option.unbind(),
            UnitGroup::Time,
            &[],
            Unit::Nanosecond,
            Unit::Hour,
            gc.reborrow(),
        )
        .unbind()?;
        temporal_rs::PlainTime::since(
            plain_time.get(agent).inner_plain_time(agent),
            &other.unbind()?,
            settings,
        )
    };
    let gc = gc.into_nogc();
    let duration = duration.map_err(|err| temporal_err_to_js_err(agent, err, gc))?;

    // 9. Return result.
    Ok(agent.heap.create(DurationRecord {
        object_index: None,
        duration,
    }))
}
