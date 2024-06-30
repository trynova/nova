use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, PromiseIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::PromiseHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Promise(pub(crate) PromiseIndex);

impl Promise {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<Promise> for PromiseIndex {
    fn from(val: Promise) -> Self {
        val.0
    }
}

impl From<PromiseIndex> for Promise {
    fn from(value: PromiseIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Promise {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Promise {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Promise> for Value {
    fn from(val: Promise) -> Self {
        Value::Promise(val)
    }
}

impl From<Promise> for Object {
    fn from(val: Promise) -> Self {
        Object::Promise(val)
    }
}

impl InternalSlots for Promise {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Promise;

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

impl InternalMethods for Promise {}

impl CreateHeapData<PromiseHeapData, Promise> for Heap {
    fn create(&mut self, data: PromiseHeapData) -> Promise {
        self.promises.push(Some(data));
        Promise(PromiseIndex::last(&self.promises))
    }
}

impl Index<Promise> for Agent {
    type Output = PromiseHeapData;

    fn index(&self, index: Promise) -> &Self::Output {
        &self.heap.promises[index]
    }
}

impl IndexMut<Promise> for Agent {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        &mut self.heap.promises[index]
    }
}

impl Index<Promise> for Vec<Option<PromiseHeapData>> {
    type Output = PromiseHeapData;

    fn index(&self, index: Promise) -> &Self::Output {
        self.get(index.get_index())
            .expect("Promise out of bounds")
            .as_ref()
            .expect("Promise slot empty")
    }
}

impl IndexMut<Promise> for Vec<Option<PromiseHeapData>> {
    fn index_mut(&mut self, index: Promise) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Promise out of bounds")
            .as_mut()
            .expect("Promise slot empty")
    }
}

impl HeapMarkAndSweep for Promise {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promises.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.promises.shift_index(&mut self.0)
    }
}
