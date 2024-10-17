// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::{
            indexed_collections::array_objects::array_iterator_objects::array_iterator::CollectionIteratorKind,
            set::Set,
        },
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::SetIteratorIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetIterator(SetIteratorIndex);

impl SetIterator {
    /// # Do not use this
    /// This is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        Self(SetIteratorIndex::from_u32_index(0))
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn from_set(agent: &mut Agent, set: Set, kind: CollectionIteratorKind) -> Self {
        agent.heap.create(SetIteratorHeapData {
            object_index: None,
            set: Some(set),
            next_index: 0,
            kind,
        })
    }
}

impl IntoValue for SetIterator {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for SetIterator {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<SetIteratorIndex> for SetIterator {
    fn from(value: SetIteratorIndex) -> Self {
        SetIterator(value)
    }
}

impl From<SetIterator> for Object {
    fn from(value: SetIterator) -> Self {
        Self::SetIterator(value)
    }
}

impl From<SetIterator> for Value {
    fn from(value: SetIterator) -> Self {
        Self::SetIterator(value)
    }
}

impl TryFrom<Value> for SetIterator {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::SetIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for SetIterator {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::SetIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl InternalSlots for SetIterator {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SetIterator;

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl InternalMethods for SetIterator {}

impl Index<SetIterator> for Agent {
    type Output = SetIteratorHeapData;

    fn index(&self, index: SetIterator) -> &Self::Output {
        &self.heap.set_iterators[index]
    }
}

impl IndexMut<SetIterator> for Agent {
    fn index_mut(&mut self, index: SetIterator) -> &mut Self::Output {
        &mut self.heap.set_iterators[index]
    }
}

impl Index<SetIterator> for Vec<Option<SetIteratorHeapData>> {
    type Output = SetIteratorHeapData;

    fn index(&self, index: SetIterator) -> &Self::Output {
        self.get(index.get_index())
            .expect("SetIterator out of bounds")
            .as_ref()
            .expect("Array SetIterator empty")
    }
}

impl IndexMut<SetIterator> for Vec<Option<SetIteratorHeapData>> {
    fn index_mut(&mut self, index: SetIterator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("SetIterator out of bounds")
            .as_mut()
            .expect("SetIterator slot empty")
    }
}

impl CreateHeapData<SetIteratorHeapData, SetIterator> for Heap {
    fn create(&mut self, data: SetIteratorHeapData) -> SetIterator {
        self.set_iterators.push(Some(data));
        SetIterator::from(SetIteratorIndex::last(&self.set_iterators))
    }
}

impl HeapMarkAndSweep for SetIterator {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.set_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.set_iterators.shift_index(&mut self.0);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SetIteratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) set: Option<Set>,
    pub(crate) next_index: usize,
    pub(crate) kind: CollectionIteratorKind,
}

impl HeapMarkAndSweep for SetIteratorHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            set,
            next_index: _,
            kind: _,
        } = self;
        object_index.mark_values(queues);
        set.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            set,
            next_index: _,
            kind: _,
        } = self;
        object_index.sweep_values(compactions);
        set.sweep_values(compactions);
    }
}
