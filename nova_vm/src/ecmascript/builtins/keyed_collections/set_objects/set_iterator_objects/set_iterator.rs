// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

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
    engine::{
        context::{Bindable, NoGcScope},
        rootable::HeapRootData,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
        indexes::SetIteratorIndex,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetIterator<'a>(SetIteratorIndex<'a>);

impl SetIterator<'_> {
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
            set: Some(set.unbind()),
            next_index: 0,
            kind,
        })
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SetIterator<'_> {
    type Of<'a> = SetIterator<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> IntoValue<'a> for SetIterator<'a> {
    fn into_value(self) -> Value<'a> {
        self.into()
    }
}

impl<'a> IntoObject<'a> for SetIterator<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<SetIterator<'a>> for Object<'a> {
    fn from(value: SetIterator) -> Self {
        Self::SetIterator(value.unbind())
    }
}

impl<'a> From<SetIterator<'a>> for Value<'a> {
    fn from(value: SetIterator<'a>) -> Self {
        Self::SetIterator(value)
    }
}

impl<'a> TryFrom<Value<'a>> for SetIterator<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::SetIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for SetIterator<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::SetIterator(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for SetIterator<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::SetIterator;

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

impl<'a> InternalMethods<'a> for SetIterator<'a> {}

impl Index<SetIterator<'_>> for Agent {
    type Output = SetIteratorHeapData;

    fn index(&self, index: SetIterator) -> &Self::Output {
        &self.heap.set_iterators[index]
    }
}

impl IndexMut<SetIterator<'_>> for Agent {
    fn index_mut(&mut self, index: SetIterator) -> &mut Self::Output {
        &mut self.heap.set_iterators[index]
    }
}

impl Index<SetIterator<'_>> for Vec<Option<SetIteratorHeapData>> {
    type Output = SetIteratorHeapData;

    fn index(&self, index: SetIterator) -> &Self::Output {
        self.get(index.get_index())
            .expect("SetIterator out of bounds")
            .as_ref()
            .expect("Array SetIterator empty")
    }
}

impl IndexMut<SetIterator<'_>> for Vec<Option<SetIteratorHeapData>> {
    fn index_mut(&mut self, index: SetIterator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("SetIterator out of bounds")
            .as_mut()
            .expect("SetIterator slot empty")
    }
}

impl TryFrom<HeapRootData> for SetIterator<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::SetIterator(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl CreateHeapData<SetIteratorHeapData, SetIterator<'static>> for Heap {
    fn create(&mut self, data: SetIteratorHeapData) -> SetIterator<'static> {
        self.set_iterators.push(Some(data));
        SetIterator(SetIteratorIndex::last(&self.set_iterators))
    }
}

impl HeapMarkAndSweep for SetIterator<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.set_iterators.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.set_iterators.shift_index(&mut self.0);
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SetIteratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject<'static>>,
    pub(crate) set: Option<Set<'static>>,
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
