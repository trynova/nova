// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{InternalMethods, InternalSlots, Object, OrdinaryObject, Value},
    },
    engine::{
        context::{Bindable, bindable_handle},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ArrayIterator<'a>(BaseIndex<'a, ArrayIteratorHeapData<'static>>);
bindable_handle!(ArrayIterator);

impl<'a> ArrayIterator<'a> {
    pub(crate) fn from_object(
        agent: &mut Agent,
        array: Object,
        kind: CollectionIteratorKind,
    ) -> Self {
        agent.heap.create(ArrayIteratorHeapData {
            object_index: None,
            array: Some(array.unbind()),
            next_index: 0,
            kind,
        })
    }
}

impl HeapIndexHandle for ArrayIterator<'_> {
    fn from_index_u32(index: u32) -> Self {
        Self(BaseIndex::from_index_u32(index))
    }

    fn get_index_u32(&self) -> u32 {
        self.0.get_index_u32()
    }
}

impl<'a> From<ArrayIterator<'a>> for Object<'a> {
    fn from(value: ArrayIterator) -> Self {
        Self::ArrayIterator(value.unbind())
    }
}

impl<'a> TryFrom<Value<'a>> for ArrayIterator<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::ArrayIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for ArrayIterator<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::ArrayIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for ArrayIterator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::ArrayIterator;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }
}

impl<'a> InternalMethods<'a> for ArrayIterator<'a> {}

impl TryFrom<HeapRootData> for ArrayIterator<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::ArrayIterator(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl<'a> CreateHeapData<ArrayIteratorHeapData<'a>, ArrayIterator<'a>> for Heap {
    fn create(&mut self, data: ArrayIteratorHeapData<'a>) -> ArrayIterator<'a> {
        self.array_iterators.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<ArrayIteratorHeapData<'static>>();
        ArrayIterator(BaseIndex::last(&self.array_iterators))
    }
}

impl HeapMarkAndSweep for ArrayIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.array_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.array_iterators.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for ArrayIterator<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .array_iterators
            .shift_weak_index(self.0)
            .map(Self)
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
pub struct ArrayIteratorHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) array: Option<Object<'a>>,
    pub(crate) next_index: i64,
    pub(crate) kind: CollectionIteratorKind,
}

bindable_handle!(ArrayIteratorHeapData);

impl HeapMarkAndSweep for ArrayIteratorHeapData<'static> {
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
