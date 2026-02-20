// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{marker::PhantomData, ptr::NonNull};

use crate::{
    ecmascript::{Agent, Value},
    engine::{
        Bindable, HeapRootCollection, HeapRootCollectionInner, NoGcScope, ScopableCollection,
        ScopedCollection,
    },
};

impl ScopableCollection for Vec<Value<'_>> {
    fn scope<'scope>(
        self,
        agent: &Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> ScopedCollection<'scope, Self::Of<'static>> {
        ScopedCollection::new(agent, self.unbind(), gc)
    }
}

impl ScopedCollection<'_, Vec<Value<'static>>> {
    fn with_cb<R>(&self, agent: &Agent, f: impl FnOnce(&Vec<Value<'static>>) -> R) -> R {
        let stack_ref_collections = agent.stack_ref_collections.borrow();
        let Some(stack_slot) = stack_ref_collections.get(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollection(HeapRootCollectionInner::ValueVec(value_vec)) = stack_slot else {
            unreachable!()
        };
        f(value_vec)
    }

    fn with_cb_mut<R>(
        &mut self,
        agent: &Agent,
        f: impl FnOnce(&mut Vec<Value<'static>>) -> R,
    ) -> R {
        let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
        let Some(stack_slot) = stack_ref_collections.get_mut(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollection(HeapRootCollectionInner::ValueVec(value_vec)) = stack_slot else {
            unreachable!()
        };
        f(value_vec)
    }

    /// Push a Value into the scoped vec.
    pub fn push(&mut self, agent: &Agent, value: Value) {
        self.with_cb_mut(agent, |value_vec| value_vec.push(value.unbind()));
    }

    /// Pop a Value from the scoped vec.
    pub fn pop<'a>(&mut self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Option<Value<'a>> {
        self.with_cb_mut(agent, |value_vec| value_vec.pop().bind(gc))
    }

    pub fn last<'a>(&self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Option<Value<'a>> {
        self.with_cb(agent, |value_vec| {
            value_vec.last().map(|value| value.bind(gc))
        })
    }

    /// Returns `true` if the scoped vec contains a Value.
    pub fn contains(&self, agent: &Agent, value: Value) -> bool {
        self.with_cb(agent, |value_vec| value_vec.contains(&value.unbind()))
    }

    pub(crate) fn iter(&self, agent: &mut Agent) -> ScopedValuesIterator<'_> {
        self.with_cb(agent, |value_vec| ScopedValuesIterator {
            slice: NonNull::from(value_vec.as_slice()),
            collection: PhantomData,
        })
    }

    pub fn is_empty(&self, agent: &Agent) -> bool {
        self.with_cb(agent, |value_vec| value_vec.is_empty())
    }

    pub fn len(&self, agent: &Agent) -> usize {
        self.with_cb(agent, |value_vec| value_vec.len())
    }
}

// SAFETY: Trivially safe.
unsafe impl<'scope> Bindable for ScopedCollection<'scope, Vec<Value<'static>>> {
    type Of<'a> = ScopedCollection<'scope, Vec<Value<'static>>>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        self
    }

    #[inline(always)]
    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        self
    }
}

#[repr(transparent)]
pub struct ScopedValuesIterator<'a> {
    slice: NonNull<[Value<'static>]>,
    collection: PhantomData<&'a [Value<'static>]>,
}

impl ScopedValuesIterator<'_> {
    pub(crate) fn from_slice(values: &[Value<'static>]) -> Self {
        Self {
            slice: NonNull::from(values),
            collection: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ScopedValue<'a> {
    key: NonNull<Value<'static>>,
    collection: PhantomData<&'a mut ScopedCollection<'a, Vec<Value<'static>>>>,
}

impl ScopedValue<'_> {
    pub fn get<'a>(&self, gc: NoGcScope<'a, '_>) -> Value<'a> {
        // SAFETY: We retain exclusive access to ScopedCollection, meaning that
        // no one else can push into the vector while we are iterating over it.
        // Garbage collection can trigger during this time which will change
        // the data in the vector, but will not reallocate it. Hence, the
        // pointer is still valid to read from and the Value in the
        // vector has been sweeped by the garbage collector if it did trigger.
        unsafe { self.key.as_ref().bind(gc) }
    }
}

impl<'a> Iterator for ScopedValuesIterator<'a> {
    type Item = ScopedValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let ScopedValuesIterator { slice, .. } = self;
        // SAFETY: We retain exclusive access to ScopedCollection, meaning that
        // no one else can push into the vector while we are iterating over it.
        // Garbage collection can trigger during this time which will change
        // the data in the vector, but will not reallocate it.
        let slice_ref = unsafe { slice.as_ref() };
        let (first, rest) = slice_ref.split_first()?;
        let first = NonNull::from(first);
        let rest = NonNull::from(rest);
        *slice = rest;
        Some(ScopedValue {
            key: first,
            collection: PhantomData,
        })
    }
}
