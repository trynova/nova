// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject, Value,
        },
    },
    engine::{context::NoGcScope, rootable::HeapRootData, Scoped},
    heap::{
        indexes::ArrayIteratorIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrayIterator<'a>(ArrayIteratorIndex<'a>);

impl ArrayIterator<'_> {
    /// Unbind this ArrayIterator from its current lifetime. This is necessary to use
    /// the ArrayIterator as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> ArrayIterator<'static> {
        unsafe { core::mem::transmute::<Self, ArrayIterator<'static>>(self) }
    }

    // Bind this ArrayIterator to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your ArrayIterators cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let array_iterator = array_iterator.bind(&gc);
    // ```
    // to make sure that the unbound ArrayIterator cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> ArrayIterator<'gc> {
        unsafe { core::mem::transmute::<Self, ArrayIterator<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, ArrayIterator<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

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
            array: Some(array.unbind()),
            next_index: 0,
            kind,
        })
    }
}

impl IntoValue for ArrayIterator<'_> {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl<'a> IntoObject<'a> for ArrayIterator<'a> {
    fn into_object(self) -> Object<'a> {
        self.into()
    }
}

impl<'a> From<ArrayIterator<'a>> for Object<'a> {
    fn from(value: ArrayIterator) -> Self {
        Self::ArrayIterator(value.unbind())
    }
}

impl From<ArrayIterator<'_>> for Value {
    fn from(value: ArrayIterator) -> Self {
        Self::ArrayIterator(value.unbind())
    }
}

impl TryFrom<Value> for ArrayIterator<'_> {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
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

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }
}

impl<'a> InternalMethods<'a> for ArrayIterator<'a> {}

impl Index<ArrayIterator<'_>> for Agent {
    type Output = ArrayIteratorHeapData;

    fn index(&self, index: ArrayIterator) -> &Self::Output {
        &self.heap.array_iterators[index]
    }
}

impl IndexMut<ArrayIterator<'_>> for Agent {
    fn index_mut(&mut self, index: ArrayIterator) -> &mut Self::Output {
        &mut self.heap.array_iterators[index]
    }
}

impl Index<ArrayIterator<'_>> for Vec<Option<ArrayIteratorHeapData>> {
    type Output = ArrayIteratorHeapData;

    fn index(&self, index: ArrayIterator) -> &Self::Output {
        self.get(index.get_index())
            .expect("ArrayIterator out of bounds")
            .as_ref()
            .expect("Array ArrayIterator empty")
    }
}

impl IndexMut<ArrayIterator<'_>> for Vec<Option<ArrayIteratorHeapData>> {
    fn index_mut(&mut self, index: ArrayIterator) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("ArrayIterator out of bounds")
            .as_mut()
            .expect("ArrayIterator slot empty")
    }
}

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

impl CreateHeapData<ArrayIteratorHeapData, ArrayIterator<'static>> for Heap {
    fn create(&mut self, data: ArrayIteratorHeapData) -> ArrayIterator<'static> {
        self.array_iterators.push(Some(data));
        ArrayIterator(ArrayIteratorIndex::last(&self.array_iterators))
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

#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum CollectionIteratorKind {
    #[default]
    Key,
    Value,
    KeyAndValue,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ArrayIteratorHeapData {
    pub(crate) object_index: Option<OrdinaryObject<'static>>,
    pub(crate) array: Option<Object<'static>>,
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
