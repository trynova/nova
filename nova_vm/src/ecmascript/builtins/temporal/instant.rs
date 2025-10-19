// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

pub(crate) mod data;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_big_int,
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
            IntoObject, IntoValue, Object, OrdinaryObject, String, Value,
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
pub(crate) struct InstantConstructor;
impl Builtin for InstantConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Instant;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(InstantConstructor::construct);
}
impl BuiltinIntrinsicConstructor for InstantConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::TemporalInstant;
}

impl InstantConstructor {
    /// ### [8.1.1 Temporal.Instant ( epochNanoseconds )](https://tc39.es/proposal-temporal/#sec-temporal.instant)
    fn construct<'gc>(
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
        create_temporal_instant(agent, epoch_nanoseconds, Some(new_target.unbind()), gc).map(
            |instant| {
                eprintln!("Temporal.Instant {:?}", &agent[instant].instant);
                instant.into_value()
            },
        )
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let instant_prototype = intrinsics.temporal_instant_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<InstantConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(instant_prototype.into_object())
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
) -> JsResult<'gc, Instant<'gc>> {
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

/// %Temporal.Instant.Prototype%
pub(crate) struct InstantPrototype;

impl InstantPrototype {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, _: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.temporal_instant_prototype();
        let object_prototype = intrinsics.object_prototype();
        let instant_constructor = intrinsics.temporal_instant();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(1) // TODO add correct property capacity
            .with_prototype(object_prototype)
            .with_constructor_property(instant_constructor)
            // TODO add all prototype methods
            .build();
    }
}

use self::data::InstantRecord;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Instant<'a>(BaseIndex<'a, InstantRecord<'static>>);
impl Instant<'_> {
    //TODO
    pub(crate) const fn _def() -> Self {
        Instant(BaseIndex::from_u32_index(0))
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
bindable_handle!(Instant);

impl<'a> From<Instant<'a>> for Value<'a> {
    fn from(value: Instant<'a>) -> Self {
        Value::Instant(value)
    }
}
impl<'a> From<Instant<'a>> for Object<'a> {
    fn from(value: Instant<'a>) -> Self {
        Object::Instant(value)
    }
}
impl<'a> TryFrom<Value<'a>> for Instant<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        match value {
            Value::Instant(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}
impl<'a> TryFrom<Object<'a>> for Instant<'a> {
    type Error = ();
    fn try_from(object: Object<'a>) -> Result<Self, ()> {
        match object {
            Object::Instant(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for Instant<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::TemporalInstant;
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }
    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl<'a> InternalMethods<'a> for Instant<'a> {}

// TODO: get rid of Index impls, replace with get/get_mut/get_direct/get_direct_mut functions
impl Index<Instant<'_>> for Agent {
    type Output = InstantRecord<'static>;

    fn index(&self, index: Instant<'_>) -> &Self::Output {
        &self.heap.instants[index]
    }
}

impl IndexMut<Instant<'_>> for Agent {
    fn index_mut(&mut self, index: Instant) -> &mut Self::Output {
        &mut self.heap.instants[index]
    }
}

impl Index<Instant<'_>> for Vec<InstantRecord<'static>> {
    type Output = InstantRecord<'static>;

    fn index(&self, index: Instant<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl IndexMut<Instant<'_>> for Vec<InstantRecord<'static>> {
    fn index_mut(&mut self, index: Instant<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("heap access out of bounds")
    }
}

impl Rootable for Instant<'_> {
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

impl HeapMarkAndSweep for Instant<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.instants.push(*self);
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.instants.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Instant<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.instants.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<InstantRecord<'a>, Instant<'a>> for Heap {
    fn create(&mut self, data: InstantRecord<'a>) -> Instant<'a> {
        self.instants.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<InstantRecord<'static>>();
        Instant(BaseIndex::last_t(&self.instants))
    }
}
