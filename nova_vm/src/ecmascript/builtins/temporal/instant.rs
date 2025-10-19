use core::ops::{Index, IndexMut};

pub(crate) mod data;

use crate::{
    ecmascript::{
        builders::{
            builtin_function_builder::BuiltinFunctionBuilder,
            ordinary_object_builder::OrdinaryObjectBuilder,
        },
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{JsResult, ProtoIntrinsics, Realm, agent::Agent},
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, IntoObject, IntoValue, Object,
            OrdinaryObject, String, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
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
    fn construct<'gc>(
        agent: &mut Agent,
        this_value: Value,
        args: ArgumentsList,
        new_target: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(agent.heap.create(InstantRecord::default()).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, gc: NoGcScope) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let instant_prototype = intrinsics.temporal_instant_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<InstantConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(instant_prototype.into_object())
            .build();
    }
}
/// %Temporal.Instant.Prototype%
pub(crate) struct InstantPrototype;

impl InstantPrototype {
    pub fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>, gc: NoGcScope) {
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
