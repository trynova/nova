// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    heap::{
        indexes::ArrayIteratorIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrayIterator(ArrayIteratorIndex);

impl ArrayIterator {
    /// # Do not use this
    /// This is only for Value discriminant creation.
    pub(crate) const fn _def() -> Self {
        Self(ArrayIteratorIndex::from_u32_index(0))
    }

    pub(crate) fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub(crate) fn from_object(
        agent: &mut Agent,
        array: Object,
        kind: CollectionIteratorKind,
    ) -> Self {
        agent.heap.create(ArrayIteratorHeapData {
            object_index: None,
            array: Some(array),
            next_index: 0,
            kind,
        })
    }
}

impl IntoValue for ArrayIterator {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for ArrayIterator {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<ArrayIteratorIndex> for ArrayIterator {
    fn from(value: ArrayIteratorIndex) -> Self {
        ArrayIterator(value)
    }
}

impl From<ArrayIterator> for Object {
    fn from(value: ArrayIterator) -> Self {
        Self::ArrayIterator(value)
    }
}

impl From<ArrayIterator> for Value {
    fn from(value: ArrayIterator) -> Self {
        Self::ArrayIterator(value)
    }
}

impl TryFrom<Value> for ArrayIterator {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::ArrayIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for ArrayIterator {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        match value {
            Object::ArrayIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl InternalSlots for ArrayIterator {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::ArrayIterator;

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }
}

impl InternalMethods for ArrayIterator {}

impl Index<ArrayIterator> for Agent {
    type Output = ArrayIteratorHeapData;

    fn index(&self, index: ArrayIterator) -> &Self::Output {
        &self.heap.array_iterators[index]
    }
}

impl IndexMut<ArrayIterator> for Agent {
    fn index_mut(&mut self, index: ArrayIterator) -> &mut Self::Output {
        &mut self.heap.array_iterators[index]
    }
}

impl Index<ArrayIterator> for Vec<Option<ArrayIteratorHeapData>> {
    type Output = ArrayIteratorHeapData;

    fn index(&self, index: ArrayIterator) -> &Self::Output {
        self.get(index.get_index())
            .expect("ArrayIterator out of bounds")
            .as_ref()
            .expect("Array ArrayIterator empty")
    }
}

impl IndexMut<ArrayIterator> for Vec<Option<ArrayIteratorHeapData>> {
    fn index_mut(&mut self, index: ArrayIterator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ArrayIterator out of bounds")
            .as_mut()
            .expect("ArrayIterator slot empty")
    }
}

impl CreateHeapData<ArrayIteratorHeapData, ArrayIterator> for Heap {
    fn create(&mut self, data: ArrayIteratorHeapData) -> ArrayIterator {
        self.array_iterators.push(Some(data));
        ArrayIterator::from(ArrayIteratorIndex::last(&self.array_iterators))
    }
}

impl HeapMarkAndSweep for ArrayIterator {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.array_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.array_iterators.shift_index(&mut self.0);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum CollectionIteratorKind {
    #[default]
    Key,
    Value,
    KeyAndValue,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ArrayIteratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) array: Option<Object>,
    pub(crate) next_index: i64,
    pub(crate) kind: CollectionIteratorKind,
}

impl HeapMarkAndSweep for ArrayIteratorHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            array,
            next_index: _,
            kind: _,
        } = self;
        object_index.mark_values(queues);
        array.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            array,
            next_index: _,
            kind: _,
        } = self;
        object_index.sweep_values(compactions);
        array.sweep_values(compactions);
    }
}
