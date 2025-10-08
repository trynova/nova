use crate::{
    ecmascript::{
        builders::{builtin_function_builder::BuiltinFunctionBuilder, ordinary_object_builder::OrdinaryObjectBuilder},
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor
        },
        execution::{agent::{Agent}, JsResult, Realm},
        types::{
            InternalSlots, IntoObject, Object, OrdinaryObject, String, Value, BUILTIN_STRING_MEMORY
        },
    },
    engine::context::{bindable_handle, GcScope, NoGcScope},
    heap::{indexes::BaseIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, IntrinsicConstructorIndexes, WorkQueues},
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
    fn construct<'gc>(agent: &mut Agent, this_value: Value, args: ArgumentsList, new_target: Option<Object>, gc: GcScope<'gc, '_>) -> JsResult<'gc, Value<'gc>> {
        todo!();
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
            .with_property_capacity(1)  // TODO add correct property capacity
            .with_prototype(object_prototype)
            .with_constructor_property(instant_constructor)
            // TODO add all prototype methods
            .build();
    }
}
/// HEAP DATA
#[derive(Debug, Clone, Copy)]
pub(crate) struct InstantValue(/*TODO:BigInt*/);

impl InstantValue {
    // TODO
}
#[derive(Debug, Clone, Copy)]
pub struct InstantHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) date: InstantValue,
}

impl InstantHeapData<'_> {
    // TODO
}

bindable_handle!(InstantHeapData);

impl HeapMarkAndSweep for InstantHeapData<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        todo!()
    }
    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        todo!()
    }
}

// HANDLES
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant<'a>(BaseIndex<'a, InstantHeapData<'static>>);
impl Instant<'_> {
    //TODO
    pub(crate) const fn _def() -> Self {
        todo!()
    }
}
bindable_handle!(Instant);

impl<'a> From<Instant<'a>> for Value<'a> {
    fn from(v: Instant<'a>) -> Self { todo!() }
}
impl<'a> From<Instant<'a>> for Object<'a> {
    fn from(v: Instant<'a>) -> Self { todo!() }
}
impl<'a> TryFrom<Value<'a>> for Instant<'a> {
    type Error = ();
    fn try_from(v: Value<'a>) -> Result<Self, ()> {
        todo!()
    }
}
impl<'a> TryFrom<Object<'a>> for Instant<'a> {
    type Error = ();
    fn try_from(o: Object<'a>) -> Result<Self, ()> {
        todo!()
    }
}

// TODO impl trait bounds properly
impl<'a> InternalSlots<'a> for Instant<'a> {
    // TODO: Add TemporalInstant to ProtoIntrinsics
    //const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::TemporalInstant;
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        todo!()
    }
    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        todo!()
    }
    
}

impl HeapMarkAndSweep for Instant<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        todo!()
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        todo!()
    }
}

impl<'a> CreateHeapData<InstantHeapData<'a>, Instant<'a>> for Heap {
    fn create(&mut self, data: InstantHeapData<'a>) -> Instant<'a> {
        todo!()
    }
}