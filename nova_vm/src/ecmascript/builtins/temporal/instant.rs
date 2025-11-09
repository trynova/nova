// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

pub(crate) mod data;
pub mod instant_constructor;
pub mod instant_prototype;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{PreferredType, to_primitive_object},
        builtins::{ordinary::ordinary_create_from_constructor, temporal::duration::to_temporal_duration},
        execution::{
            JsResult, ProtoIntrinsics,
            agent::{Agent, ExceptionType},
        },
        types::{
            Function, InternalMethods, InternalSlots, IntoFunction, IntoValue, Object, OrdinaryObject, Primitive, String, Value
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable, Scopable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
    },
};

use self::data::InstantHeapData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TemporalInstant<'a>(BaseIndex<'a, InstantHeapData<'static>>);

impl TemporalInstant<'_> {
    pub(crate) fn inner_instant(self, agent: &Agent) -> temporal_rs::Instant {
        agent[self].instant
    }

    //TODO
    pub(crate) const fn _def() -> Self {
        TemporalInstant(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// # Safety
    ///
    /// Should be only called once; reinitialising the value is not allowed.
    unsafe fn set_epoch_nanoseconds(
        self,
        agent: &mut Agent,
        epoch_nanoseconds: temporal_rs::Instant,
    ) {
        agent[self].instant = epoch_nanoseconds;
    }
}
bindable_handle!(TemporalInstant);

impl<'a> From<TemporalInstant<'a>> for Value<'a> {
    fn from(value: TemporalInstant<'a>) -> Self {
        Value::Instant(value)
    }
}
impl<'a> From<TemporalInstant<'a>> for Object<'a> {
    fn from(value: TemporalInstant<'a>) -> Self {
        Object::Instant(value)
    }
}
impl<'a> TryFrom<Value<'a>> for TemporalInstant<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        match value {
            Value::Instant(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}
impl<'a> TryFrom<Object<'a>> for TemporalInstant<'a> {
    type Error = ();
    fn try_from(object: Object<'a>) -> Result<Self, ()> {
        match object {
            Object::Instant(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for TemporalInstant<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::TemporalInstant;
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }
    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for TemporalInstant<'a> {}

// TODO: get rid of Index impls, replace with get/get_mut/get_direct/get_direct_mut functions
impl Index<TemporalInstant<'_>> for Agent {
    type Output = InstantHeapData<'static>;

    fn index(&self, index: TemporalInstant<'_>) -> &Self::Output {
        &self.heap.instants[index]
    }
}

impl IndexMut<TemporalInstant<'_>> for Agent {
    fn index_mut(&mut self, index: TemporalInstant) -> &mut Self::Output {
        &mut self.heap.instants[index]
    }
}

impl Index<TemporalInstant<'_>> for Vec<InstantHeapData<'static>> {
    type Output = InstantHeapData<'static>;

    fn index(&self, index: TemporalInstant<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl IndexMut<TemporalInstant<'_>> for Vec<InstantHeapData<'static>> {
    fn index_mut(&mut self, index: TemporalInstant<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl Rootable for TemporalInstant<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Instant(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Instant(object) => Some(object),
            _ => None,
        }
    }
}

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

impl<'a> CreateHeapData<InstantHeapData<'a>, TemporalInstant<'a>> for Heap {
    fn create(&mut self, data: InstantHeapData<'a>) -> TemporalInstant<'a> {
        self.instants.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<InstantHeapData<'static>>();
        TemporalInstant(BaseIndex::last_t(&self.instants))
    }
}

/// 8.5.2 CreateTemporalInstant ( epochNanoseconds [ , newTarget ] )
///
/// The abstract operation CreateTemporalInstant takes argument
/// epochNanoseconds (a BigInt) and optional argument newTarget (a constructor)
/// and returns either a normal completion containing a Temporal.Instant or a
/// throw completion. It creates a Temporal.Instant instance and fills the
/// internal slots with valid values.
fn create_temporal_instant<'gc>(
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
            .into_function()
    });
    // 3. Let object be ? OrdinaryCreateFromConstructor(newTarget, "%Temporal.Instant.prototype%", « [[InitializedTemporalInstant]], [[EpochNanoseconds]] »).
    let Object::Instant(object) =
        ordinary_create_from_constructor(agent, new_target, ProtoIntrinsics::TemporalInstant, gc)?
    else {
        unreachable!()
    };
    // 4. Set object.[[EpochNanoseconds]] to epochNanoseconds.
    // SAFETY: initialising Instant.
    unsafe { object.set_epoch_nanoseconds(agent, epoch_nanoseconds) };
    // 5. Return object.
    Ok(object)
}

/// ### [8.5.3 ToTemporalInstant ( item )](https://tc39.es/proposal-temporal/#sec-temporal-totemporalinstant)
///
/// The abstract operation ToTemporalInstant takes argument item (an ECMAScript language value) and
/// returns either a normal completion containing a Temporal.Instant or a throw completion.
/// Converts item to a new Temporal.Instant instance if possible, and throws otherwise.
fn to_temporal_instant<'gc>(
    agent: &mut Agent,
    item: Value,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::Instant> {
    let item = item.bind(gc.nogc());
    // 1. If item is an Object, then
    let item = if let Ok(item) = Object::try_from(item) {
        // a. If item has an [[InitializedTemporalInstant]] or [[InitializedTemporalZonedDateTime]]
        // internal slot, then TODO: TemporalZonedDateTime::try_from(item)
        if let Ok(item) = TemporalInstant::try_from(item) {
            // i. Return ! CreateTemporalInstant(item.[[EpochNanoseconds]]).
            return Ok(agent[item].instant);
        }
        // b. NOTE: This use of ToPrimitive allows Instant-like objects to be converted.
        // c. Set item to ? ToPrimitive(item, string).
        to_primitive_object(agent, item.unbind(), Some(PreferredType::String), gc)?
    } else {
        Primitive::try_from(item).unwrap()
    };
    // 2. If item is not a String, throw a TypeError exception.
    let Ok(item) = String::try_from(item) else {
        todo!() // TypeErrror
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
    let parsed = temporal_rs::Instant::from_utf8(item.as_bytes(agent)).unwrap();
    Ok(parsed)
}

/// [8.5.10 AddDurationToInstant ( operation, instant, temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal-adddurationtoinstant)
/// The abstract operation AddDurationToInstant takes arguments operation
/// (add or subtract), instant (a Temporal.Instant),
/// and temporalDurationLike (an ECMAScript language value)
/// and returns either a normal completion containing a Temporal.Instant
/// or a throw completion.
/// It adds/subtracts temporalDurationLike to/from instant.
fn add_duration_to_instant<'gc, const IS_ADD: bool>(
    agent: &mut Agent,
    instant: TemporalInstant,
    duration: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let duration = duration.bind(gc.nogc());
    let instant = instant.bind(gc.nogc());
    // 1. Let duration be ? ToTemporalDuration(temporalDurationLike).
    let instant = instant.scope(agent, gc.nogc());
    let duration = to_temporal_duration(agent, duration.unbind(), gc.reborrow());
    // 2. If operation is subtract, set duration to CreateNegatedTemporalDuration(duration).
    // 3. Let largestUnit be DefaultTemporalLargestUnit(duration).
    // 4. If TemporalUnitCategory(largestUnit) is date, throw a RangeError exception.
    // 5. Let internalDuration be ToInternalDurationRecordWith24HourDays(duration).
    // 6. Let ns be ? AddInstant(instant.[[EpochNanoseconds]], internalDuration.[[Time]]).
    let ns_result = if IS_ADD {
        temporal_rs::Instant::add(&agent[instant.get(agent)].instant, &duration.unwrap()).unwrap()
    } else {
        temporal_rs::Instant::subtract(&agent[instant.get(agent)].instant, &duration.unwrap())
            .unwrap()
    };
    // 7. Return ! CreateTemporalInstant(ns).
    let instant = create_temporal_instant(agent, ns_result, None, gc)?;
    Ok(instant.into_value())
}

/// [8.5.9 DifferenceTemporalInstant ( operation, instant, other, options )]()
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
    instant: Value,
    other: Value,
    options: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let instant = instant.bind(gc.nogc());
    let other = other.bind(gc.nogc());
    let options = options.bind(gc.nogc());
    // 1. Set other to ? ToTemporalInstant(other).
    let other = to_temporal_instant(agent, other.unbind(), gc.reborrow());
    // 2. Let resolvedOptions be ? GetOptionsObject(options).
    // 3. Let settings be ? GetDifferenceSettings(operation, resolvedOptions, time, « », nanosecond, second).
    // 4. Let internalDuration be DifferenceInstant(instant.[[EpochNanoseconds]], other.[[EpochNanoseconds]], settings.[[RoundingIncrement]], settings.[[SmallestUnit]], settings.[[RoundingMode]]).
    // 5. Let result be ! TemporalDurationFromInternal(internalDuration, settings.[[LargestUnit]]).
    // 6. If operation is since, set result to CreateNegatedTemporalDuration(result).
    // 7. Return result.
    unimplemented!()
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
