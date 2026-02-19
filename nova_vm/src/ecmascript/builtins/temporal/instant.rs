// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;
mod instant_constructor;
mod instant_prototype;

pub(crate) use data::*;
pub(crate) use instant_constructor::*;
pub(crate) use instant_prototype::*;

use temporal_rs::options::{Unit, UnitGroup};

use crate::{
    ecmascript::{
        Agent, DurationRecord, ExceptionType, Function, InternalMethods, InternalSlots, JsResult,
        Object, OrdinaryObject, PreferredType, Primitive, ProtoIntrinsics, String,
        TemporalDuration, Value, get_difference_settings, get_options_object, object_handle,
        ordinary_populate_from_constructor, temporal_err_to_js_err, to_primitive_object,
        to_temporal_duration,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable},
    heap::{
        ArenaAccess, ArenaAccessMut, BaseIndex, CompactionLists, CreateHeapData, Heap,
        HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TemporalInstant<'a>(BaseIndex<'a, InstantRecord<'static>>);
object_handle!(TemporalInstant, Instant);
arena_vec_access!(
    TemporalInstant,
    'a,
    InstantRecord,
    instants
);

impl TemporalInstant<'_> {
    pub(crate) fn inner_instant(self, agent: &Agent) -> &temporal_rs::Instant {
        &self.unbind().get(agent).instant
    }
}

impl<'a> InternalSlots<'a> for TemporalInstant<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::TemporalInstant;
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

impl<'a> InternalMethods<'a> for TemporalInstant<'a> {}

impl HeapMarkAndSweep for TemporalInstant<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.instants.push(*self);
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.instants.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for TemporalInstant<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.instants.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<InstantRecord<'a>, TemporalInstant<'a>> for Heap {
    fn create(&mut self, data: InstantRecord<'a>) -> TemporalInstant<'a> {
        self.instants.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<InstantRecord<'static>>();
        TemporalInstant(BaseIndex::last(&self.instants))
    }
}

/// 8.5.2 CreateTemporalInstant ( epochNanoseconds [ , newTarget ] )
///
/// The abstract operation CreateTemporalInstant takes argument
/// epochNanoseconds (a BigInt) and optional argument newTarget (a constructor)
/// and returns either a normal completion containing a Temporal.Instant or a
/// throw completion. It creates a Temporal.Instant instance and fills the
/// internal slots with valid values.
pub(crate) fn create_temporal_instant<'gc>(
    agent: &mut Agent,
    epoch_nanoseconds: temporal_rs::Instant,
    new_target: Option<Function>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, TemporalInstant<'gc>> {
    // 1. Assert: IsValidEpochNanoseconds(epochNanoseconds) is true.
    // 2. If newTarget is not present, set newTarget to %Temporal.Instant%.
    let new_target = new_target.unwrap_or_else(|| {
        agent
            .current_realm_record()
            .intrinsics()
            .temporal_instant()
            .into()
    });
    // 3. Let object be ? OrdinaryCreateFromConstructor(newTarget,
    // "%Temporal.Instant.prototype%", « [[InitializedTemporalInstant]],
    // [[EpochNanoseconds]] »).
    let object = agent.heap.create(InstantRecord {
        object_index: None,
        // 4. Set object.[[EpochNanoseconds]] to epochNanoseconds.
        instant: epoch_nanoseconds,
    });
    // 5. Return object.
    Ok(
        TemporalInstant::try_from(ordinary_populate_from_constructor(
            agent,
            object.unbind().into(),
            new_target,
            ProtoIntrinsics::TemporalInstant,
            gc,
        )?)
        .unwrap(),
    )
}

/// ### [8.5.3 ToTemporalInstant ( item )](https://tc39.es/proposal-temporal/#sec-temporal-totemporalinstant)
///
/// The abstract operation ToTemporalInstant takes argument item (an ECMAScript language value) and
/// returns either a normal completion containing a Temporal.Instant or a throw completion.
/// Converts item to a new Temporal.Instant instance if possible, and throws otherwise.
pub(crate) fn to_temporal_instant<'gc>(
    agent: &mut Agent,
    item: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::Instant> {
    let item = item.bind(gc.nogc());
    // 1. If item is an Object, then
    let item = if let Ok(item) = Object::try_from(item) {
        // a. If item has an [[InitializedTemporalInstant]] or [[InitializedTemporalZonedDateTime]]
        // internal slot, then
        // TODO: TemporalZonedDateTime::try_from(item)
        if let Ok(item) = TemporalInstant::try_from(item) {
            // i. Return ! CreateTemporalInstant(item.[[EpochNanoseconds]]).
            return Ok(*item.inner_instant(agent));
        }
        // b. NOTE: This use of ToPrimitive allows Instant-like objects to be converted.
        // c. Set item to ? ToPrimitive(item, string).
        to_primitive_object(
            agent,
            item.unbind(),
            Some(PreferredType::String),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc())
    } else {
        Primitive::try_from(item).unwrap()
    };
    // 2. If item is not a String, throw a TypeError exception.
    let Ok(item) = String::try_from(item) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Item is not a String",
            gc.into_nogc(),
        ));
    };
    // 3. Let parsed be ? ParseISODateTime(item, « TemporalInstantString »).
    // 4. Assert: Either parsed.[[TimeZone]].[[OffsetString]] is not empty or
    //    parsed.[[TimeZone]].[[Z]] is true, but not both.
    // 5. If parsed.[[TimeZone]].[[Z]] is true, let offsetNanoseconds be 0; otherwise, let
    //    offsetNanoseconds be ! ParseDateTimeUTCOffset(parsed.[[TimeZone]].[[OffsetString]]).
    // 6. If parsed.[[Time]] is start-of-day, let time be MidnightTimeRecord(); else let time be
    //    parsed.[[Time]].
    // 7. Let balanced be BalanceISODateTime(parsed.[[Year]], parsed.[[Month]], parsed.[[Day]],
    //    time.[[Hour]], time.[[Minute]], time.[[Second]], time.[[Millisecond]],
    //    time.[[Microsecond]], time.[[Nanosecond]] - offsetNanoseconds).
    // 8. Perform ? CheckISODaysRange(balanced.[[ISODate]]).
    // 9. Let epochNanoseconds be GetUTCEpochNanoseconds(balanced).
    // 10. If IsValidEpochNanoseconds(epochNanoseconds) is false, throw a RangeError exception.
    // 11. Return ! CreateTemporalInstant(epochNanoseconds).
    temporal_rs::Instant::from_utf8(item.as_bytes(agent))
        .map_err(|e| temporal_err_to_js_err(agent, e, gc.into_nogc()))
}

/// [8.5.10 AddDurationToInstant ( operation, instant, temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal-adddurationtoinstant)
///
/// The abstract operation AddDurationToInstant takes arguments operation (add
/// or subtract), instant (a Temporal.Instant), and temporalDurationLike (an
/// ECMAScript language value) and returns either a normal completion containing
/// a Temporal.Instant or a throw completion. It adds/subtracts
/// temporalDurationLike to/from instant.
fn add_duration_to_instant<'gc, const IS_ADD: bool>(
    agent: &mut Agent,
    instant: TemporalInstant,
    duration: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, TemporalInstant<'gc>> {
    let duration = duration.bind(gc.nogc());
    let mut instant = instant.bind(gc.nogc());
    // 1. Let duration be ? ToTemporalDuration(temporalDurationLike).

    let duration = if let Value::Duration(duration) = duration {
        duration.get(agent).duration
    } else {
        let scoped_instant = instant.scope(agent, gc.nogc());
        let res = to_temporal_duration(agent, duration.unbind(), gc.reborrow()).unbind()?;
        // SAFETY: not shared
        unsafe {
            instant = scoped_instant.take(agent);
        }
        res
    };

    // 2. If operation is subtract, set duration to CreateNegatedTemporalDuration(duration).
    // 3. Let largestUnit be DefaultTemporalLargestUnit(duration).
    // 4. If TemporalUnitCategory(largestUnit) is date, throw a RangeError exception.
    // 5. Let internalDuration be ToInternalDurationRecordWith24HourDays(duration).
    // 6. Let ns be ? AddInstant(instant.[[EpochNanoseconds]], internalDuration.[[Time]]).
    let ns_result = if IS_ADD {
        temporal_rs::Instant::add(instant.inner_instant(agent), &duration)
            .map_err(|err| temporal_err_to_js_err(agent, err, gc.nogc()))
            .unbind()?
    } else {
        temporal_rs::Instant::subtract(instant.inner_instant(agent), &duration)
            .map_err(|err| temporal_err_to_js_err(agent, err, gc.nogc()))
            .unbind()?
    };
    // 7. Return ! CreateTemporalInstant(ns).
    Ok(create_temporal_instant(agent, ns_result, None, gc).unwrap())
}

/// [8.5.9 DifferenceTemporalInstant ( operation, instant, other, options )](https://tc39.es/proposal-temporal/#sec-temporal-differencetemporalinstant)
/// The abstract operation DifferenceTemporalInstant takes arguments
/// operation (since or until), instant (a Temporal.Instant),
/// other (an ECMAScript language value), and options
/// (an ECMAScript language value) and returns either
/// a normal completion containing a Temporal.Duration or a
/// throw completion. It computes the difference between the
/// two times represented by instant and other, optionally
/// rounds it, and returns it as a Temporal.Duration object.
fn difference_temporal_instant<'gc, const IS_UNTIL: bool>(
    agent: &mut Agent,
    instant: TemporalInstant,
    other: Value,
    options: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, TemporalDuration<'gc>> {
    let instant = instant.scope(agent, gc.nogc());
    let other = other.bind(gc.nogc());
    let options = options.scope(agent, gc.nogc());
    // 1. Set other to ? ToTemporalInstant(other).
    let other = to_temporal_instant(agent, other.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    // 2. Let resolvedOptions be ? GetOptionsObject(options).
    let resolved_options = get_options_object(agent, options.get(agent), gc.nogc())
        .unbind()?
        .bind(gc.nogc());
    // 3. Let settings be ? GetDifferenceSettings(operation, resolvedOptions,
    //    time, « », nanosecond, second).
    // 4. Let internalDuration be
    //    DifferenceInstant(instant.[[EpochNanoseconds]],
    //    other.[[EpochNanoseconds]], settings.[[RoundingIncrement]],
    //    settings.[[SmallestUnit]], settings.[[RoundingMode]]).
    // 5. Let result be ! TemporalDurationFromInternal(internalDuration,
    //    settings.[[LargestUnit]]).
    // 6. If operation is since, set result to
    //    CreateNegatedTemporalDuration(result).
    let duration = if IS_UNTIL {
        const UNTIL: bool = true;
        let settings = get_difference_settings::<UNTIL>(
            agent,
            resolved_options.unbind(),
            UnitGroup::Time,
            &[],
            Unit::Nanosecond,
            Unit::Second,
            gc.reborrow(),
        )
        .unbind()?;
        temporal_rs::Instant::until(instant.get(agent).inner_instant(agent), &other, settings)
    } else {
        const SINCE: bool = false;
        let settings = get_difference_settings::<SINCE>(
            agent,
            resolved_options.unbind(),
            UnitGroup::Time,
            &[],
            Unit::Nanosecond,
            Unit::Second,
            gc.reborrow(),
        )
        .unbind()?;
        temporal_rs::Instant::since(instant.get(agent).inner_instant(agent), &other, settings)
    };
    let gc = gc.into_nogc();
    let duration = duration.map_err(|err| temporal_err_to_js_err(agent, err, gc))?;

    // 7. Return result.
    Ok(agent.heap.create(DurationRecord {
        object_index: None,
        duration,
    }))
}

#[inline(always)]
fn require_internal_slot_temporal_instant<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, TemporalInstant<'a>> {
    match value {
        Value::Instant(instant) => Ok(instant.bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Object is not a Temporal Instant",
            gc,
        )),
    }
}
