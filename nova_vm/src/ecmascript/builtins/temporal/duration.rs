use temporal_rs::partial::PartialDuration;

use crate::{ecmascript::{builtins::temporal::instant::TemporalInstant, execution::{Agent, JsResult, agent::ExceptionType}, types::{InternalMethods, Object, Value}}, engine::context::{Bindable, GcScope, NoGcScope, bindable_handle}, heap::indexes::BaseIndex};
use core::ops::{Index, IndexMut};

pub(crate) mod data;
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
    pub(crate) fn inner_duration(self, agent: &Agent) -> temporal_rs::Duration {
        agent[self].duration
    }
}

bindable_handle!(TemporalDuration);


impl<'a> From<TemporalDuration<'a>> for Value<'a> {
    fn from(value: TemporalDuration<'a>) -> Self {
        todo!()
        //Value::Duration(value)
    }
}
impl<'a> From<TemporalDuration<'a>> for Object<'a> {
    fn from(value: TemporalDuration<'a>) -> Self {
        todo!()
        //Object::Duration(value)
    }
}
impl<'a> TryFrom<Value<'a>> for TemporalDuration<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        todo!()
        // match value {
        //     Value::Duration(idx) => Ok(idx),
        //     _ => Err(()),
        // }
    }
}

impl Index<TemporalDuration<'_>> for Agent {
    type Output = DurationHeapData<'static>;

    fn index(&self, _index: TemporalDuration<'_>) -> &Self::Output {
        unimplemented!()
        //&self.heap.durations[index]
    }
}

impl IndexMut<TemporalDuration<'_>> for Agent {
    fn index_mut(&mut self, _index: TemporalDuration) -> &mut Self::Output {
        unimplemented!()
        //&mut self.heap.durations[index]
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
/// 7.5.19 CreateTemporalDuration ( years, months, weeks,
/// days, hours, minutes, seconds,
/// milliseconds, microseconds, nanoseconds [ , newTarget ] )
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
/// It performs the following steps when called:
fn create_temporal_duration<'gc> (
    // years,
    // months,
    // weeks,
    // days,
    // hours,
    // minutes,
    // seconds,
    // milliseconds,
    // microseconds,
    // nanoseconds: ,
    // new_target: Option<Function>,
) -> JsResult<'gc, TemporalDuration<'gc>> {
    // 1. If IsValidDuration(years, months, weeks, days, hours, minutes, seconds, milliseconds, microseconds, nanoseconds) is false, throw a RangeError exception.
    // 2. If newTarget is not present, set newTarget to %Temporal.Duration%.
    // 3. Let object be ? OrdinaryCreateFromConstructor(newTarget, "%Temporal.Duration.prototype%", « [[InitializedTemporalDuration]], [[Years]], [[Months]], [[Weeks]], [[Days]], [[Hours]], [[Minutes]], [[Seconds]], [[Milliseconds]], [[Microseconds]], [[Nanoseconds]] »).
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
pub(crate) fn to_temporal_duration<'gc> (
    agent: &mut Agent,
    item: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::Duration> {
    let item = item.bind(gc.nogc());
    // 1. If item is an Object and item has an [[InitializedTemporalDuration]] internal slot, then
    
    if let Ok(obj) = require_internal_slot_temporal_duration(agent, item, gc.nogc()) {
        // `require_internal_slot_temporal_duration` already guarantees this is a Duration object,
        let obj = Object::try_from(obj);
        unimplemented!();
        // a. Return ! CreateTemporalDuration(item.[[Years]], item.[[Months]], item.[[Weeks]], item.[[Days]], item.[[Hours]], item.[[Minutes]], item.[[Seconds]], item.[[Milliseconds]], item.[[Microseconds]], item.[[Nanoseconds]]).
    }
    // 2. If item is not an Object, then
    if let Ok(item) = item.unbind().to_string(agent, gc.reborrow()){
        // b. Return ? ParseTemporalDurationString(item).
        let parsed = temporal_rs::Duration::from_utf8(item.as_bytes(agent)).unwrap();
        return Ok(parsed)
    } else {
        // a. If item is not a String, throw a TypeError exception.
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "item is not a string",
            gc.into_nogc(),
        ));
    }
    // 3. Let result be a new Partial Duration Record with each field set to 0.
    // 4. Let partial be ? ToTemporalPartialDurationRecord(item).
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
    unimplemented!()
}

/// [7.5.20 CreateNegatedTemporalDuration ( duration )] (https://tc39.es/proposal-temporal/#sec-temporal-createnegatedtemporalduration)
/// The abstract operation CreateNegatedTemporalDuration takes argument 
/// duration (a Temporal.Duration) and returns a Temporal.Duration. 
/// It returns a new Temporal.Duration instance that is the 
/// negation of duration. 
pub(crate) fn create_negated_temporal_duration<'gc> (
    agent: &mut Agent,
    item: temporal_rs::Duration,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, temporal_rs::Duration> {
    // 1. Return ! CreateTemporalDuration(-duration.[[Years]], -duration.[[Months]], -duration.[[Weeks]], -duration.[[Days]], -duration.[[Hours]], -duration.[[Minutes]], -duration.[[Seconds]], -duration.[[Milliseconds]], -duration.[[Microseconds]], -duration.[[Nanoseconds]]).
    let duration = temporal_rs::Duration::negated(&item);
    //TODO: IMPL create_temporal_duration() 
    unimplemented!()
}

#[inline(always)]
fn require_internal_slot_temporal_duration<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
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