pub(crate) mod data;
pub mod duration_constructor;
pub mod duration_prototype;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::get, type_conversion::to_integer_if_integral,
        },
        builtins::ordinary::ordinary_create_from_constructor,
        execution::{Agent, JsResult, ProtoIntrinsics, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, InternalMethods, InternalSlots, IntoFunction, Object,
            OrdinaryObject, String, Value,
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
use core::ops::{Index, IndexMut};

use self::data::DurationHeapData;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TemporalDuration<'a>(BaseIndex<'a, DurationHeapData<'static>>);

impl TemporalDuration<'_> {
    pub(crate) const fn _def() -> Self {
        TemporalDuration(BaseIndex::from_u32_index(0))
    }
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
    // pub(crate) fn inner_duration(self, agent: &Agent) -> temporal_rs::Duration {
    //     agent[self].duration
    // }
}

bindable_handle!(TemporalDuration);

impl<'a> From<TemporalDuration<'a>> for Value<'a> {
    fn from(value: TemporalDuration<'a>) -> Self {
        Value::Duration(value)
    }
}
impl<'a> From<TemporalDuration<'a>> for Object<'a> {
    fn from(value: TemporalDuration<'a>) -> Self {
        Object::Duration(value)
    }
}
impl<'a> TryFrom<Value<'a>> for TemporalDuration<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        match value {
            Value::Duration(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for TemporalDuration<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::TemporalDuration;
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }
    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for TemporalDuration<'a> {}

impl Index<TemporalDuration<'_>> for Agent {
    type Output = DurationHeapData<'static>;

    fn index(&self, index: TemporalDuration<'_>) -> &Self::Output {
        &self.heap.durations[index]
    }
}

impl IndexMut<TemporalDuration<'_>> for Agent {
    fn index_mut(&mut self, index: TemporalDuration) -> &mut Self::Output {
        &mut self.heap.durations[index]
    }
}

impl Index<TemporalDuration<'_>> for Vec<DurationHeapData<'static>> {
    type Output = DurationHeapData<'static>;

    fn index(&self, index: TemporalDuration<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl IndexMut<TemporalDuration<'_>> for Vec<DurationHeapData<'static>> {
    fn index_mut(&mut self, index: TemporalDuration<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl Rootable for TemporalDuration<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Duration(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Duration(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for TemporalDuration<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.durations.push(*self);
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.durations.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for TemporalDuration<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.durations.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<DurationHeapData<'a>, TemporalDuration<'a>> for Heap {
    fn create(&mut self, data: DurationHeapData<'a>) -> TemporalDuration<'a> {
        self.durations.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<DurationHeapData<'static>>();
        TemporalDuration(BaseIndex::last(&self.durations))
    }
}
/// [7.5.19 CreateTemporalDuration ( years, months, weeks, days, hours, minutes, seconds, milliseconds, microseconds, nanoseconds [ , newTarget ] )](https://tc39.es/proposal-temporal/#sec-temporal-createtemporalduration)
/// The abstract operation CreateTemporalDuration takes arguments
/// years (an integer), months (an integer),
/// weeks (an integer), days (an integer),
/// hours (an integer), minutes (an integer),
/// seconds (an integer), milliseconds (an integer),
/// microseconds (an integer), and nanoseconds (an integer)
/// and optional argument newTarget (a constructor)
/// and returns either a normal completion containing
/// a Temporal.Duration or a throw completion.
/// It creates a Temporal.Duration instance and fills
/// the internal slots with valid values.
pub(crate) fn create_temporal_duration<'gc>(
    // years,
    agent: &mut Agent,
    duration: temporal_rs::Duration,
    new_target: Option<Function>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, TemporalDuration<'gc>> {
    // 1. If IsValidDuration(years, months, weeks, days, hours, minutes, seconds, milliseconds, microseconds, nanoseconds) is false, throw a RangeError exception.
    // 2. If newTarget is not present, set newTarget to %Temporal.Duration%.
    let new_target = new_target.unwrap_or_else(|| {
        agent
            .current_realm_record()
            .intrinsics()
            .temporal_duration()
            .into_function()
    });
    // 3. Let object be ? OrdinaryCreateFromConstructor(newTarget, "%Temporal.Duration.prototype%", « [[InitializedTemporalDuration]], [[Years]], [[Months]], [[Weeks]], [[Days]], [[Hours]], [[Minutes]], [[Seconds]], [[Milliseconds]], [[Microseconds]], [[Nanoseconds]] »).
    let Object::Duration(object) =
        ordinary_create_from_constructor(agent, new_target, ProtoIntrinsics::TemporalInstant, gc)?
    else {
        unreachable!()
    };
    // 4. Set object.[[Years]] to ℝ(𝔽(years)).
    // 5. Set object.[[Months]] to ℝ(𝔽(months)).
    // 6. Set object.[[Weeks]] to ℝ(𝔽(weeks)).
    // 7. Set object.[[Days]] to ℝ(𝔽(days)).
    // 8. Set object.[[Hours]] to ℝ(𝔽(hours)).
    // 9. Set object.[[Minutes]] to ℝ(𝔽(minutes)).
    // 10. Set object.[[Seconds]] to ℝ(𝔽(seconds)).
    // 11. Set object.[[Milliseconds]] to ℝ(𝔽(milliseconds)).
    // 12. Set object.[[Microseconds]] to ℝ(𝔽(microseconds)).
    // 13. Set object.[[Nanoseconds]] to ℝ(𝔽(nanoseconds)).
    // 14. Return object.
    unimplemented!()
}

/// Abstract Operations <--->

/// [7.5.12 ToTemporalDuration ( item )](https://tc39.es/proposal-temporal/#sec-temporal-totemporalduration)
/// The abstract operation ToTemporalDuration takes argument item
/// (an ECMAScript language value) and returns either a normal completion containing a
/// Temporal.Duration or a throw completion. Converts item to a new Temporal.Duration
/// instance if possible and returns that, and throws otherwise.
/// It performs the following steps when called:
pub(crate) fn to_temporal_duration<'gc>(
    agent: &mut Agent,
    item: Value,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::Duration> {
    let item = item.bind(gc.nogc());
    // 1. If item is an Object and item has an [[InitializedTemporalDuration]] internal slot, then
    if let Ok(_obj) = require_internal_slot_temporal_duration(agent, item, gc.nogc()) {
        unimplemented!();
        // a. Return ! CreateTemporalDuration(item.[[Years]], item.[[Months]], item.[[Weeks]], item.[[Days]], item.[[Hours]], item.[[Minutes]], item.[[Seconds]], item.[[Milliseconds]], item.[[Microseconds]], item.[[Nanoseconds]]).
    }
    // 2. If item is not an Object, then
    if !item.is_object() {
        let Ok(item) = String::try_from(item) else {
            // a. If item is not a String, throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "item is not a string",
                gc.into_nogc(),
            ));
        };
        // b. Return ? ParseTemporalDurationString(item).
        let parsed = temporal_rs::Duration::from_utf8(item.as_bytes(agent)).unwrap();
        return Ok(parsed);
    }
    // 3. Let result be a new Partial Duration Record with each field set to 0.
    // 4. Let partial be ? ToTemporalPartialDurationRecord(item).
    let partial =
        to_temporal_partial_duration_record(agent, Object::try_from(item).unwrap().unbind(), gc);
    // 5. If partial.[[Years]] is not undefined, set result.[[Years]] to partial.[[Years]].
    // 6. If partial.[[Months]] is not undefined, set result.[[Months]] to partial.[[Months]].
    // 7. If partial.[[Weeks]] is not undefined, set result.[[Weeks]] to partial.[[Weeks]].
    // 8. If partial.[[Days]] is not undefined, set result.[[Days]] to partial.[[Days]].
    // 9. If partial.[[Hours]] is not undefined, set result.[[Hours]] to partial.[[Hours]].
    // 10. If partial.[[Minutes]] is not undefined, set result.[[Minutes]] to partial.[[Minutes]].
    // 11. If partial.[[Seconds]] is not undefined, set result.[[Seconds]] to partial.[[Seconds]].
    // 12. If partial.[[Milliseconds]] is not undefined, set result.[[Milliseconds]] to partial.[[Milliseconds]].
    // 13. If partial.[[Microseconds]] is not undefined, set result.[[Microseconds]] to partial.[[Microseconds]].
    // 14. If partial.[[Nanoseconds]] is not undefined, set result.[[Nanoseconds]] to partial.[[Nanoseconds]].
    // 15. Return ? CreateTemporalDuration(result.[[Years]], result.[[Months]], result.[[Weeks]], result.[[Days]], result.[[Hours]], result.[[Minutes]], result.[[Seconds]], result.[[Milliseconds]], result.[[Microseconds]], result.[[Nanoseconds]]).
    Ok(temporal_rs::Duration::from_partial_duration(partial.unwrap()).unwrap())
}

/// [7.5.18 ToTemporalPartialDurationRecord ( temporalDurationLike )](https://tc39.es/proposal-temporal/#sec-temporal-totemporalpartialdurationrecord)
/// The abstract operation ToTemporalPartialDurationRecord takes argument temporalDurationLike
/// (an ECMAScript language value) and returns either a normal completion containing a
/// partial Duration Record or a throw completion. The returned Record has its fields
/// set according to the properties of temporalDurationLike.
pub(crate) fn to_temporal_partial_duration_record<'gc>(
    agent: &mut Agent,
    temporal_duration_like: Object,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::partial::PartialDuration> {
    let temporal_duration_like = temporal_duration_like.scope(agent, gc.nogc());
    // 1. If temporalDurationLike is not an Object, then
    //    a. Throw a TypeError exception.
    // 2. Let result be a new partial Duration Record with each field set to undefined.
    let mut result = temporal_rs::partial::PartialDuration::empty();
    // 3. NOTE: The following steps read properties and perform independent validation in alphabetical order.
    // 4. Let days be ? Get(temporalDurationLike, "days").
    let days = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.days.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 5. If days is not undefined, set result.[[Days]] to ? ToIntegerIfIntegral(days).
    if !days.is_undefined() {
        let days = to_integer_if_integral(agent, days.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.days = Some(days.into_i64(agent))
    }
    // 6. Let hours be ? Get(temporalDurationLike, "hours").
    let hours = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.hours.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 7. If hours is not undefined, set result.[[Hours]] to ? ToIntegerIfIntegral(hours).
    if !hours.is_undefined() {
        let hours = to_integer_if_integral(agent, hours.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.hours = Some(hours.into_i64(agent))
    }
    // 8. Let microseconds be ? Get(temporalDurationLike, "microseconds").
    let microseconds = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.microseconds.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 9. If microseconds is not undefined, set result.[[Microseconds]] to ? ToIntegerIfIntegral(microseconds).
    if !microseconds.is_undefined() {
        let microseconds = to_integer_if_integral(agent, microseconds.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.microseconds = Some(microseconds.into_i64(agent) as i128);
    }
    // 10. Let milliseconds be ? Get(temporalDurationLike, "milliseconds").
    let milliseconds = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.milliseconds.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 11. If milliseconds is not undefined, set result.[[Milliseconds]] to ? ToIntegerIfIntegral(milliseconds).
    if !milliseconds.is_undefined() {
        let milliseconds = to_integer_if_integral(agent, milliseconds.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.milliseconds = Some(milliseconds.into_i64(agent))
    }
    // 12. Let minutes be ? Get(temporalDurationLike, "minutes").
    let minutes = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.minutes.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 13. If minutes is not undefined, set result.[[Minutes]] to ? ToIntegerIfIntegral(minutes).
    if !minutes.is_undefined() {
        let minutes = to_integer_if_integral(agent, minutes.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.minutes = Some(minutes.into_i64(agent))
    }
    // 14. Let months be ? Get(temporalDurationLike, "months").
    let months = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.months.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 15. If months is not undefined, set result.[[Months]] to ? ToIntegerIfIntegral(months).
    if !months.is_undefined() {
        let months = to_integer_if_integral(agent, months.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.months = Some(months.into_i64(agent))
    }
    // 16. Let nanoseconds be ? Get(temporalDurationLike, "nanoseconds").
    let nanoseconds = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.nanoseconds.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 17. If nanoseconds is not undefined, set result.[[Nanoseconds]] to ? ToIntegerIfIntegral(nanoseconds).
    if !nanoseconds.is_undefined() {
        let nanoseconds = to_integer_if_integral(agent, nanoseconds.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.nanoseconds = Some(nanoseconds.into_i64(agent) as i128);
    }
    // 18. Let seconds be ? Get(temporalDurationLike, "seconds").
    let seconds = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.seconds.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 19. If seconds is not undefined, set result.[[Seconds]] to ? ToIntegerIfIntegral(seconds).
    if !seconds.is_undefined() {
        let seconds = to_integer_if_integral(agent, seconds.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.seconds = Some(seconds.into_i64(agent))
    }
    // 20. Let weeks be ? Get(temporalDurationLike, "weeks").
    let weeks = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.weeks.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 21. If weeks is not undefined, set result.[[Weeks]] to ? ToIntegerIfIntegral(weeks).
    if !weeks.is_undefined() {
        let weeks = to_integer_if_integral(agent, weeks.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.weeks = Some(weeks.into_i64(agent))
    }
    // 22. Let years be ? Get(temporalDurationLike, "years").
    let years = get(
        agent,
        temporal_duration_like.get(agent),
        BUILTIN_STRING_MEMORY.years.to_property_key(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 23. If years is not undefined, set result.[[Years]] to ? ToIntegerIfIntegral(years).
    if !years.is_undefined() {
        let years = to_integer_if_integral(agent, years.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        result.years = Some(years.into_i64(agent))
    }
    // 24. If years is undefined, and months is undefined, and weeks is undefined, and days is undefined, and hours is undefined, and minutes is undefined, and seconds is undefined, and milliseconds is undefined, and microseconds is undefined, and nanoseconds is undefined, throw a TypeError exception.
    if result.years.is_none()
        && result.months.is_none()
        && result.weeks.is_none()
        && result.days.is_none()
        && result.hours.is_none()
        && result.minutes.is_none()
        && result.seconds.is_none()
        && result.milliseconds.is_none()
        && result.microseconds.is_none()
        && result.nanoseconds.is_none()
    {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Duration must have at least one unit",
            gc.into_nogc(),
        ));
    }
    // 25. Return result.
    Ok(result)
}

/// [7.5.20 CreateNegatedTemporalDuration ( duration )] (https://tc39.es/proposal-temporal/#sec-temporal-createnegatedtemporalduration)
/// The abstract operation CreateNegatedTemporalDuration takes argument
/// duration (a Temporal.Duration) and returns a Temporal.Duration.
/// It returns a new Temporal.Duration instance that is the
/// negation of duration.
pub(crate) fn _create_negated_temporal_duration<'gc>(
    _agent: &mut Agent,
    _item: temporal_rs::Duration,
    mut _gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::Duration> {
    // 1. Return ! CreateTemporalDuration(-duration.[[Years]], -duration.[[Months]], -duration.[[Weeks]], -duration.[[Days]], -duration.[[Hours]], -duration.[[Minutes]], -duration.[[Seconds]], -duration.[[Milliseconds]], -duration.[[Microseconds]], -duration.[[Nanoseconds]]).
    unimplemented!()
}

#[inline(always)]
fn require_internal_slot_temporal_duration<'a>(
    _agent: &mut Agent,
    _value: Value,
    _gc: NoGcScope<'a, '_>,
) -> JsResult<'a, TemporalDuration<'a>> {
    unimplemented!()
    // TODO:
    // match value {
    //     Value::Instant(instant) => Ok(instant.bind(gc)),
    //     _ => Err(agent.throw_exception_with_static_message(
    //         ExceptionType::TypeError,
    //         "Object is not a Temporal Instant",
    //         gc,
    //     )),
    // }
}
