use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObjectInternalSlots, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, FinalizationRegistryIndex},
        CreateHeapData, Heap,
    },
};

use self::data::FinalizationRegistryHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct FinalizationRegistry(pub(crate) FinalizationRegistryIndex);

impl FinalizationRegistry {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<FinalizationRegistry> for FinalizationRegistryIndex {
    fn from(val: FinalizationRegistry) -> Self {
        val.0
    }
}

impl From<FinalizationRegistryIndex> for FinalizationRegistry {
    fn from(value: FinalizationRegistryIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for FinalizationRegistry {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for FinalizationRegistry {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<FinalizationRegistry> for Value {
    fn from(val: FinalizationRegistry) -> Self {
        Value::FinalizationRegistry(val)
    }
}

impl From<FinalizationRegistry> for Object {
    fn from(val: FinalizationRegistry) -> Self {
        Object::FinalizationRegistry(val)
    }
}

impl OrdinaryObjectInternalSlots for FinalizationRegistry {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::FinalizationRegistry;

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

impl InternalMethods for FinalizationRegistry {}

impl Index<FinalizationRegistry> for Agent {
    type Output = FinalizationRegistryHeapData;

    fn index(&self, index: FinalizationRegistry) -> &Self::Output {
        self.heap
            .finalization_registrys
            .get(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_ref()
            .expect("FinalizationRegistry slot empty")
    }
}

impl IndexMut<FinalizationRegistry> for Agent {
    fn index_mut(&mut self, index: FinalizationRegistry) -> &mut Self::Output {
        self.heap
            .finalization_registrys
            .get_mut(index.get_index())
            .expect("FinalizationRegistry out of bounds")
            .as_mut()
            .expect("FinalizationRegistry slot empty")
    }
}

impl CreateHeapData<FinalizationRegistryHeapData, FinalizationRegistry> for Heap {
    fn create(&mut self, data: FinalizationRegistryHeapData) -> FinalizationRegistry {
        self.finalization_registrys.push(Some(data));
        FinalizationRegistry(FinalizationRegistryIndex::last(
            &self.finalization_registrys,
        ))
    }
}
