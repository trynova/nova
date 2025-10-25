// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

pub(crate) mod data;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{PreferredType, to_big_int, to_primitive_object},
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
            ordinary::ordinary_create_from_constructor,
        },
        execution::{
            JsResult, ProtoIntrinsics, Realm,
            agent::{Agent, ExceptionType},
        },
        types::{
            BUILTIN_STRING_MEMORY, BigInt, Function, InternalMethods, InternalSlots, IntoFunction,
            IntoObject, IntoValue, Object, OrdinaryObject, Primitive, String, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable, Scopable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        IntrinsicConstructorIndexes, WorkQueues, indexes::BaseIndex,
    },
};

/// Constructor function object for %Temporal.Instant%.
pub(crate) struct TemporalInstantConstructor;

impl Builtin for TemporalInstantConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Instant;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(TemporalInstantConstructor::constructor);
}

impl BuiltinIntrinsicConstructor for TemporalInstantConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalInstant;
}

struct TemporalInstantFrom;
impl Builtin for TemporalInstantFrom {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.from;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantConstructor::from);
}

struct TemporalInstantFromEpochMilliseconds;
impl Builtin for TemporalInstantFromEpochMilliseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fromEpochMilliseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(TemporalInstantConstructor::from_epoch_milliseconds);
}

struct TemporalInstantFromEpochNanoseconds;
impl Builtin for TemporalInstantFromEpochNanoseconds {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fromEpochNanoseconds;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(TemporalInstantConstructor::from_epoch_nanoseconds);
}

struct TemporalInstantCompare;
impl Builtin for TemporalInstantCompare {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.compare;
    const LENGTH: u8 = 2;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(TemporalInstantConstructor::compare);
}

impl TemporalInstantConstructor {
    /// ### [8.1.1 Temporal.Instant ( epochNanoseconds )](https://tc39.es/proposal-temporal/#sec-temporal.instant)
    fn constructor<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let epoch_nanoseconds = args.get(0).bind(gc.nogc());
        let new_target = new_target.bind(gc.nogc());
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin Temporal.Instant constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };
        let Ok(mut new_target) = Function::try_from(new_target) else {
            unreachable!()
        };
        // 2. Let epochNanoseconds be ? ToBigInt(epochNanoseconds).
        let epoch_nanoseconds = if let Ok(epoch_nanoseconds) = BigInt::try_from(epoch_nanoseconds) {
            epoch_nanoseconds
        } else {
            let scoped_new_target = new_target.scope(agent, gc.nogc());
            let epoch_nanoseconds = to_big_int(agent, epoch_nanoseconds.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: not shared.
            new_target = unsafe { scoped_new_target.take(agent) }.bind(gc.nogc());
            epoch_nanoseconds
        };

        // 3. If IsValidEpochNanoseconds(epochNanoseconds) is false, throw a RangeError exception.
        let Some(epoch_nanoseconds) = epoch_nanoseconds
            .try_into_i128(agent)
            .and_then(|nanoseconds| temporal_rs::Instant::try_new(nanoseconds).ok())
        else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "value out of range",
                gc.into_nogc(),
            ));
        };
        // 4. Return ? CreateTemporalInstant(epochNanoseconds, NewTarget).
        create_temporal_instant(agent, epoch_nanoseconds, Some(new_target.unbind()), gc)
            .map(|instant| instant.into_value())
    }

    /// ### [8.2.2 Temporal.Instant.from ( item )](https://tc39.es/proposal-temporal/#sec-temporal.instant.from)
    fn from<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let item = args.get(0).bind(gc.nogc());
        // 1. Return ? ToTemporalInstant(item).
        let instant = to_temporal_instant(agent, item.unbind(), gc)?;
        let instant = agent.heap.create(InstantRecord {
            object_index: None,
            instant,
        });
        Ok(instant.into_value())
    }

    /// ### [8.2.3 Temporal.Instant.fromEpochMilliseconds ( epochMilliseconds )](https://tc39.es/proposal-temporal/#sec-temporal.instant.fromepochmilliseconds)
    fn from_epoch_milliseconds<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let epoch_ms = args.get(0).bind(gc.nogc());
        // 1. Set epochMilliseconds to ? ToNumber(epochMilliseconds).
        let epoch_ms_number = epoch_ms
            .unbind()
            .to_number(agent, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 2. Set epochMilliseconds to ? NumberToBigInt(epochMilliseconds).
        if !epoch_ms_number.is_integer(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Can't convert number to BigInt because it isn't an integer",
                gc.into_nogc(),
            ));
        }
        // 3. Let epochNanoseconds be epochMilliseconds × ℤ(10**6).
        // 4. If IsValidEpochNanoseconds(epochNanoseconds) is false, throw a RangeError exception.
        let epoch_ns =
            match temporal_rs::Instant::from_epoch_milliseconds(epoch_ms_number.into_i64(agent)) {
                Ok(instant) => instant,
                Err(_) => {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::RangeError,
                        "epochMilliseconds value out of range",
                        gc.into_nogc(),
                    ));
                }
            };

        // 5. Return ! CreateTemporalInstant(epochNanoseconds).
        let instant = create_temporal_instant(agent, epoch_ns, None, gc)?;
        let value = instant.into_value();
        Ok(value)
    }

    /// [8.2.4 Temporal.Instant.fromEpochNanoseconds ( epochNanoseconds )] (https://tc39.es/proposal-temporal/#sec-temporal.instant.fromepochnanoseconds)
    fn from_epoch_nanoseconds<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let epoch_nanoseconds = arguments.get(0).bind(gc.nogc());
        // 1. Set epochNanoseconds to ? ToBigInt(epochNanoseconds).
        let epoch_nanoseconds = if let Ok(epoch_nanoseconds) = BigInt::try_from(epoch_nanoseconds) {
            epoch_nanoseconds
        } else {
            let epoch_nanoseconds = to_big_int(agent, epoch_nanoseconds.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            epoch_nanoseconds
        };
        // 2. If IsValidEpochNanoseconds(epochNanoseconds) is false, throw a RangeError exception.
        let Some(epoch_nanoseconds) = epoch_nanoseconds
            .try_into_i128(agent)
            .and_then(|nanoseconds| temporal_rs::Instant::try_new(nanoseconds).ok())
        else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "epochNanoseconds",
                gc.into_nogc(),
            ));
        };
        // 3. Return ! CreateTemporalInstant(epochNanoseconds).
        let instant = create_temporal_instant(agent, epoch_nanoseconds, None, gc)?;
        let value = instant.into_value();
        Ok(value)
    }

    /// ### [8.2.5 Temporal.Instant.compare ( one, two )](https://tc39.es/proposal-temporal/#sec-temporal.instant.compare)
    fn compare<'gc>(
        agent: &mut Agent,
        _: Value,
        args: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let one = args.get(0).bind(gc.nogc());
        let two = args.get(0).bind(gc.nogc());
        let two = two.scope(agent, gc.nogc());
        // 1. Set one to ? ToTemporalInstant(one).
        let one_instant = to_temporal_instant(agent, one.unbind(), gc.reborrow()).unbind()?;
        // 2. Set two to ? ToTemporalInstant(two).
        let two_value = two.get(agent).bind(gc.nogc());
        let two_instant = to_temporal_instant(agent, two_value.unbind(), gc.reborrow()).unbind()?;
        // 3. Return 𝔽(CompareEpochNanoseconds(one.[[EpochNanoseconds]], two.[[EpochNanoseconds]])).
        Ok((one_instant.cmp(&two_instant) as i8).into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _gc: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let instant_prototype = intrinsics.temporal_instant_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<TemporalInstantConstructor>(
            agent, realm,
        )
        .with_property_capacity(5)
        .with_prototype_property(instant_prototype.into_object())
        .with_builtin_function_property::<TemporalInstantFrom>()
        .with_builtin_function_property::<TemporalInstantFromEpochMilliseconds>()
        .with_builtin_function_property::<TemporalInstantFromEpochNanoseconds>()
        .with_builtin_function_property::<TemporalInstantCompare>()
        .build();
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
/// Converts item to a new Temporal.Instant instance if possible, and throws otherwise. It performs
/// the following steps when called:
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

/// %Temporal.Instant.Prototype%
pub(crate) struct TemporalInstantPrototype;

impl TemporalInstantPrototype {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_instant_prototype();
        let object_prototype = intrinsics.object_prototype();
        let instant_constructor = intrinsics.temporal_instant();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(1)
            .with_prototype(object_prototype)
            .with_constructor_property(instant_constructor)
            .build();
    }
}

use self::data::InstantRecord;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TemporalInstant<'a>(BaseIndex<'a, InstantRecord<'static>>);
impl TemporalInstant<'_> {
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
    type Output = InstantRecord<'static>;

    fn index(&self, index: TemporalInstant<'_>) -> &Self::Output {
        &self.heap.instants[index]
    }
}

impl IndexMut<TemporalInstant<'_>> for Agent {
    fn index_mut(&mut self, index: TemporalInstant) -> &mut Self::Output {
        &mut self.heap.instants[index]
    }
}

impl Index<TemporalInstant<'_>> for Vec<InstantRecord<'static>> {
    type Output = InstantRecord<'static>;

    fn index(&self, index: TemporalInstant<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl IndexMut<TemporalInstant<'_>> for Vec<InstantRecord<'static>> {
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

impl<'a> CreateHeapData<InstantRecord<'a>, TemporalInstant<'a>> for Heap {
    fn create(&mut self, data: InstantRecord<'a>) -> TemporalInstant<'a> {
        self.instants.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<InstantRecord<'static>>();
        TemporalInstant(BaseIndex::last_t(&self.instants))
    }
}
