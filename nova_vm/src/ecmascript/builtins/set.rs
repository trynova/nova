use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, ObjectHeapData, OrdinaryObject,
            OrdinaryObjectInternalSlots, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, SetIndex},
        CompactionLists, CreateHeapData, HeapMarkAndSweep, ObjectEntry, WorkQueues,
    },
    Heap,
};

use self::data::SetHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Set(pub(crate) SetIndex);

impl Set {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<Set> for SetIndex {
    fn from(val: Set) -> Self {
        val.0
    }
}

impl From<SetIndex> for Set {
    fn from(value: SetIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Set {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Set {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Set> for Value {
    fn from(val: Set) -> Self {
        Value::Set(val)
    }
}

impl From<Set> for Object {
    fn from(val: Set) -> Self {
        Object::Set(val)
    }
}

fn create_set_base_object(agent: &mut Agent, set: Set, entries: &[ObjectEntry]) -> OrdinaryObject {
    // TODO: An issue crops up if multiple realms are in play:
    // The prototype should not be dependent on the realm we're operating in
    // but should instead be bound to the realm the object was created in.
    // We'll have to cross this bridge at a later point, likely be designating
    // a "default realm" and making non-default realms always initialize ObjectHeapData.
    let prototype = agent.current_realm().intrinsics().set_prototype();
    let object_index = agent
        .heap
        .create_object_with_prototype(prototype.into(), entries);
    agent[set].object_index = Some(object_index);
    object_index
}

impl OrdinaryObjectInternalSlots for Set {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Set;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
        let prototype = agent
            .current_realm()
            .intrinsics()
            .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE);
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys: Default::default(),
            values: Default::default(),
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }
}

impl InternalMethods for Set {}

impl HeapMarkAndSweep for Set {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.sets.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = SetIndex::from_u32(self_index - compactions.sets.get_shift_for_index(self_index));
    }
}

impl Index<Set> for Agent {
    type Output = SetHeapData;

    fn index(&self, index: Set) -> &Self::Output {
        self.heap
            .sets
            .get(index.get_index())
            .expect("Set out of bounds")
            .as_ref()
            .expect("Set slot empty")
    }
}

impl IndexMut<Set> for Agent {
    fn index_mut(&mut self, index: Set) -> &mut Self::Output {
        self.heap
            .sets
            .get_mut(index.get_index())
            .expect("Set out of bounds")
            .as_mut()
            .expect("Set slot empty")
    }
}

impl CreateHeapData<SetHeapData, Set> for Heap {
    fn create(&mut self, data: SetHeapData) -> Set {
        self.sets.push(Some(data));
        Set(SetIndex::last(&self.sets))
    }
}
