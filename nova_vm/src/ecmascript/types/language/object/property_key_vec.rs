// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{marker::PhantomData, ptr::NonNull};

use crate::{
    ecmascript::Agent,
    engine::{Bindable, HeapRootCollection, NoGcScope, ScopableCollection, ScopedCollection},
};

use super::PropertyKey;

impl ScopableCollection for Vec<PropertyKey<'_>> {
    fn scope<'scope>(
        self,
        agent: &Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> ScopedCollection<'scope, Self::Of<'static>> {
        ScopedCollection::new(agent, self.unbind(), gc)
    }
}

impl ScopedCollection<'_, Vec<PropertyKey<'static>>> {
    fn with_cb<R>(&self, agent: &Agent, f: impl FnOnce(&Vec<PropertyKey<'static>>) -> R) -> R {
        let stack_ref_collections = agent.stack_ref_collections.borrow();
        let Some(stack_slot) = stack_ref_collections.get(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollection::PropertyKeyVec(property_key_vec) = stack_slot else {
            unreachable!()
        };
        f(property_key_vec)
    }

    fn with_cb_mut<R>(
        &mut self,
        agent: &Agent,
        f: impl FnOnce(&mut Vec<PropertyKey<'static>>) -> R,
    ) -> R {
        let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
        let Some(stack_slot) = stack_ref_collections.get_mut(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollection::PropertyKeyVec(property_key_vec) = stack_slot else {
            unreachable!()
        };
        f(property_key_vec)
    }

    /// Push a PropertyKey into the scoped vec.
    pub fn push(&mut self, agent: &Agent, value: PropertyKey) {
        self.with_cb_mut(agent, |property_key_vec| {
            property_key_vec.push(value.unbind())
        });
    }

    /// Returns `true` if the scoped vec contains a PropertyKey.
    pub fn contains(&self, agent: &Agent, value: PropertyKey) -> bool {
        self.with_cb(agent, |property_key_vec| {
            property_key_vec.contains(&value.unbind())
        })
    }

    pub(crate) fn iter(&self, agent: &mut Agent) -> ScopedPropertyKeysIterator<'_> {
        self.with_cb(agent, |property_key_vec| ScopedPropertyKeysIterator {
            slice: NonNull::from(property_key_vec.as_slice()),
            collection: PhantomData,
        })
    }

    /// Returns `true` if the scoped vec is empty.
    pub fn is_empty(&self, agent: &Agent) -> bool {
        self.with_cb(agent, |property_key_vec| property_key_vec.is_empty())
    }

    /// Returns the length of the scoped vec.
    pub fn len(&self, agent: &Agent) -> usize {
        self.with_cb(agent, |property_key_vec| property_key_vec.len())
    }
}

// SAFETY: Trivially safe.
unsafe impl<'scope> Bindable for ScopedCollection<'scope, Vec<PropertyKey<'static>>> {
    type Of<'a> = ScopedCollection<'scope, Vec<PropertyKey<'static>>>;

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
pub(crate) struct ScopedPropertyKeysIterator<'a> {
    slice: NonNull<[PropertyKey<'static>]>,
    collection: PhantomData<&'a mut ScopedCollection<'a, Vec<PropertyKey<'static>>>>,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub(crate) struct ScopedPropertyKey<'a> {
    key: NonNull<PropertyKey<'static>>,
    collection: PhantomData<&'a mut ScopedCollection<'a, Vec<PropertyKey<'static>>>>,
}

impl ScopedPropertyKey<'_> {
    /// Get the referenced PropertyKey.
    pub fn get<'a>(self, gc: NoGcScope<'a, '_>) -> PropertyKey<'a> {
        // SAFETY: We retain exclusive access to ScopedCollection, meaning that
        // no one else can push into the vector while we are iterating over it.
        // Garbage collection can trigger during this time which will change
        // the data in the vector, but will not reallocate it. Hence, the
        // pointer is still valid to read from and the PropertyKey in the
        // vector has been sweeped by the garbage collector if it did trigger.
        unsafe { self.key.as_ref().bind(gc) }
    }
}

impl<'a> Iterator for ScopedPropertyKeysIterator<'a> {
    type Item = ScopedPropertyKey<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let ScopedPropertyKeysIterator { slice, .. } = self;
        // SAFETY: We retain exclusive access to ScopedCollection, meaning that
        // no one else can push into the vector while we are iterating over it.
        // Garbage collection can trigger during this time which will change
        // the data in the vector, but will not reallocate it.
        let slice_ref = unsafe { slice.as_ref() };
        let (first, rest) = slice_ref.split_first()?;
        let first = NonNull::from(first);
        let rest = NonNull::from(rest);
        *slice = rest;
        Some(ScopedPropertyKey {
            key: first,
            collection: PhantomData,
        })
    }
}
