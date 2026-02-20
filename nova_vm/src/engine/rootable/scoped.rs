// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::marker::PhantomData;

use crate::{
    ecmascript::Agent,
    engine::{
        Bindable, HeapRootCollectionData, HeapRootDataInner, HeapRootRef, NoGcScope, Rootable,
        ScopeToken,
    },
};

use super::{HeapRootData, RootableCollection};

/// # Scoped heap root
///
/// This type roots a heap-allocated JavaScript engine value for the duration
/// of the current JavaScript call context, roughly corresponding to a native
/// call scope. Stack-allocated values avoid rooting. Rooted values cannot be
/// garbage collected, so accessing the rooted value is always safe within the
/// current call context. This type is intended for cheap rooting of JavaScript
/// Values that need to be used after calling into functions that may trigger
/// garbage collection.
#[derive(Hash, Clone)]
#[repr(transparent)]
pub struct Scoped<'a, T: 'static + Rootable> {
    pub(crate) inner: T::RootRepr,
    _marker: PhantomData<T>,
    _scope: PhantomData<&'a ScopeToken>,
}

impl<T: 'static + Rootable> core::fmt::Debug for Scoped<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Scoped<{}>", core::any::type_name::<T>())
    }
}

impl<T: 'static + Rootable> Scoped<'static, T> {
    #[inline(always)]
    pub(crate) const fn from_root_repr(value: T::RootRepr) -> Scoped<'static, T> {
        Self {
            inner: value,
            _marker: PhantomData,
            _scope: PhantomData,
        }
    }
}

pub trait Scopable: Rootable + Bindable
where
    for<'a> Self::Of<'a>: Rootable + Bindable,
{
    fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, Self::Of<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }
}

impl<T: Rootable + Bindable> Scopable for T where for<'a> Self::Of<'a>: Rootable + Bindable {}

impl<'scope, T: Rootable> Scoped<'scope, T> {
    /// Unwrap the Scoped value to get access to the inner RootRepr value of
    /// the wrapped type.
    ///
    /// ## Safety
    ///
    /// The RootRepr does not carry the 'scope lifetime and is thus liable to
    /// become use-after-free. This method should only be used to implement eg.
    /// trivial From-implementations, or TryFrom-like methods.
    pub(crate) unsafe fn into_root_repr(self) -> T::RootRepr {
        self.inner
    }

    pub fn new(agent: &Agent, value: T, _gc: NoGcScope<'_, 'scope>) -> Self {
        let value = match T::to_root_repr(value) {
            Ok(stack_repr) => {
                // The value doesn't need rooting.
                return Self {
                    inner: stack_repr,
                    _marker: PhantomData,
                    _scope: PhantomData,
                };
            }
            Err(heap_data) => heap_data,
        };
        let mut stack_refs = agent.stack_refs.borrow_mut();
        let next_index = stack_refs.len();
        stack_refs.push(value);
        Self {
            inner: T::from_heap_ref(HeapRootRef::from_index(next_index)),
            _marker: PhantomData,
            _scope: PhantomData,
        }
    }

    /// Returns the scoped value from the heap. If the scoped value was at the
    /// top of the scope stack, then this will drop the value from the stack.
    ///
    /// ## Safety
    ///
    /// The scoped value should not be shared with any other piece of code that
    /// is still going to reuse it.
    ///
    /// ## Panics
    ///
    /// If the scoped value has been taken by another caller already, the
    /// method panics.
    #[must_use]
    pub unsafe fn take(self, agent: &Agent) -> T {
        match T::from_root_repr(&self.inner) {
            Ok(value) => value,
            Err(heap_root_ref) => {
                let index = heap_root_ref.to_index();
                let mut stack_refs = agent.stack_refs.borrow_mut();
                let Some(heap_data) = stack_refs.get_mut(index) else {
                    handle_bound_check_failure()
                };
                let heap_data =
                    core::mem::replace(heap_data, HeapRootData(HeapRootDataInner::Empty));
                if index == stack_refs.len() - 1 {
                    Self::drop_empty_slots(&mut stack_refs);
                }
                let Some(value) = T::from_heap_data(heap_data) else {
                    handle_invalid_scoped_conversion()
                };
                value
            }
        }
    }

    /// Internal helper function to drop empty slots from the stack. This
    /// method is separate as dropping empty slots should be a reasonably
    /// rare operation.
    fn drop_empty_slots(stack_refs: &mut Vec<HeapRootData>) {
        // We just replaced the last item with an Empty, so we can
        // shorten the stack by at least one slot.
        let last_non_empty_index = stack_refs
            .iter()
            .enumerate()
            .rfind(|(_, v)| !matches!(v.0, HeapRootDataInner::Empty))
            .map_or(0, |(index, _)| index + 1);
        debug_assert!(last_non_empty_index < stack_refs.len());
        // SAFETY: The last non-empty index is necessarily within
        // the bounds of the vector, so this only shortens it.
        unsafe { stack_refs.set_len(last_non_empty_index) };
    }

    pub fn get(&self, agent: &Agent) -> T {
        match T::from_root_repr(&self.inner) {
            Ok(value) => value,
            Err(heap_root_ref) => {
                let Some(&heap_data) = agent.stack_refs.borrow().get(heap_root_ref.to_index())
                else {
                    handle_bound_check_failure()
                };
                let Some(value) = T::from_heap_data(heap_data) else {
                    handle_invalid_scoped_conversion()
                };
                value
            }
        }
    }

    // TODO: Make this const once from_root_repr can be made const.
    // For now the inline(always) is our way to hope that this works equally.
    /// Unwrap the Scoped wrapper, exposing the on-stack value contained
    /// within.
    ///
    /// ## Panics
    ///
    /// If the contained value is a heap reference, the method panics.
    #[inline(always)]
    pub fn unwrap(&self) -> T {
        let Ok(value) = T::from_root_repr(&self.inner) else {
            unreachable!("Scoped value was a heap reference")
        };
        value
    }

    /// Replace an existing scoped value on the heap with a new value of the
    /// same type.
    ///
    /// ## Safety
    ///
    /// If the scoped value has been cloned and is still being used, replacing
    /// its value will be observable to the other users and they will likely
    /// find this unexpected.
    ///
    /// This method should only ever be called on scoped values that have not
    /// been shared outside the caller.
    pub unsafe fn replace(&mut self, agent: &Agent, value: T) {
        let heap_data = match T::to_root_repr(value) {
            Ok(stack_repr) => {
                // The value doesn't need rooting.
                let previous = core::mem::replace(
                    self,
                    Self {
                        inner: stack_repr,
                        _marker: PhantomData,
                        _scope: PhantomData,
                    },
                );

                // Let's take the previous value from the heap if it existed.
                // SAFETY: The caller guarantees that the scoped value has not
                // been shared.
                let _ = unsafe { previous.take(agent) };
                return;
            }
            Err(heap_data) => heap_data,
        };
        match T::from_root_repr(&self.inner) {
            Ok(_) => {
                // We do not have an existing slot but now need one.
                let mut stack_refs = agent.stack_refs.borrow_mut();
                let next_index = stack_refs.len();
                stack_refs.push(heap_data);
                *self = Self {
                    inner: T::from_heap_ref(HeapRootRef::from_index(next_index)),
                    _marker: PhantomData,
                    _scope: PhantomData,
                }
            }
            Err(heap_root_ref) => {
                // Existing slot, we can just replace the data.
                let mut stack_refs_borrow = agent.stack_refs.borrow_mut();
                let Some(heap_slot) = stack_refs_borrow.get_mut(heap_root_ref.to_index()) else {
                    handle_bound_check_failure()
                };
                *heap_slot = heap_data;
            }
        }
    }

    /// Replace an existing scoped value on the heap with a new value of a
    /// different type.
    ///
    /// ## Safety
    ///
    /// If the scoped value has been cloned and is still being used, replacing
    /// its value will be observable to the other users and they will likely
    /// find this unexpected and will likely panic from a type mismatch.
    ///
    /// This method should only ever be called on scoped values that have not
    /// been shared outside the caller.
    pub unsafe fn replace_self<U: 'static + Rootable>(
        self,
        agent: &mut Agent,
        value: U,
    ) -> Scoped<'scope, U> {
        let heap_data = match U::to_root_repr(value) {
            Ok(stack_repr) => {
                // Let's take the previous value from the heap if it existed.
                // SAFETY: The caller guarantees that the scoped value has not
                // been shared.
                let _ = unsafe { self.take(agent) };
                // The value doesn't need rooting.
                return Scoped {
                    inner: stack_repr,
                    _marker: PhantomData,
                    _scope: PhantomData,
                };
            }
            Err(heap_data) => heap_data,
        };
        match T::from_root_repr(&self.inner) {
            Ok(_) => {
                // The previous scoped value did not have an heap slot but now
                // need one.
                let mut stack_refs = agent.stack_refs.borrow_mut();
                let next_index = stack_refs.len();
                stack_refs.push(heap_data);
                Scoped {
                    inner: U::from_heap_ref(HeapRootRef::from_index(next_index)),
                    _marker: PhantomData,
                    _scope: PhantomData,
                }
            }
            Err(heap_root_ref) => {
                // Existing slot, we can just replace the data.
                let mut stack_refs_borrow = agent.stack_refs.borrow_mut();
                let Some(heap_slot) = stack_refs_borrow.get_mut(heap_root_ref.to_index()) else {
                    handle_bound_check_failure()
                };
                *heap_slot = heap_data;
                Scoped {
                    inner: U::from_heap_ref(heap_root_ref),
                    _marker: PhantomData,
                    _scope: PhantomData,
                }
            }
        }
    }
}

pub trait ScopableCollection: Bindable
where
    Self::Of<'static>: RootableCollection,
{
    fn scope<'scope>(
        self,
        agent: &Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> ScopedCollection<'scope, Self::Of<'static>>;
}

/// # Scoped heap root collection
#[derive(Debug, Hash, Clone)]
#[repr(transparent)]
pub struct ScopedCollection<'a, T: 'static + RootableCollection> {
    /// Index to Agent's stack_ref_collections
    pub(crate) inner: u32,
    _marker: PhantomData<T>,
    _scope: PhantomData<&'a ScopeToken>,
}

impl<'a, T: 'static + RootableCollection> ScopedCollection<'a, T> {
    /// Create a new ScopedCollection by moving a rootable collection onto the
    /// Agent's heap.
    pub(crate) fn new(agent: &Agent, rootable: T, _gc: NoGcScope<'_, 'a>) -> Self {
        let heap_data = rootable.to_heap_data();
        let inner = u32::try_from(agent.stack_ref_collections.borrow().len())
            .expect("ScopedCollections stack overflowed");
        agent.stack_ref_collections.borrow_mut().push(heap_data);
        Self {
            inner,
            _marker: PhantomData,
            _scope: PhantomData,
        }
    }

    /// Take ownership of the rootable collection from the Agent's heap.
    #[must_use]
    pub(crate) fn take(self, agent: &Agent) -> T {
        let index = self.inner;
        let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
        let heap_slot = stack_ref_collections.get_mut(index as usize).unwrap();
        let heap_data = core::mem::replace(heap_slot, HeapRootCollectionData::Empty);
        if index as usize == stack_ref_collections.len() - 1 {
            Self::drop_empty_slots(&mut stack_ref_collections);
        }
        T::from_heap_data(heap_data)
    }

    /// Internal helper function to drop empty slots from the stack. This
    /// method is separate as dropping empty slots should be a reasonably
    /// rare operation.
    fn drop_empty_slots(stack_ref_collections: &mut Vec<HeapRootCollectionData>) {
        // We replaced the last stack item with an Empty, so we can shorten
        // the stack by at least one.
        let last_non_empty_index = stack_ref_collections
            .iter()
            .enumerate()
            .rfind(|(_, v)| !matches!(v, HeapRootCollectionData::Empty))
            .map_or(0, |(index, _)| index + 1);
        debug_assert!(last_non_empty_index < stack_ref_collections.len());
        // SAFETY: The last non-empty index is necessarily within
        // the bounds of the vector, so this only shortens it. The
        // items being dropped are also Empty slots which don't
        // need any drop calls, so this is not a memory leak
        // either.
        unsafe { stack_ref_collections.set_len(last_non_empty_index) };
    }
}

#[cold]
#[inline(never)]
fn handle_invalid_scoped_conversion() -> ! {
    panic!("Attempted to convert mismatched Scoped");
}

#[cold]
#[inline(never)]
fn handle_bound_check_failure() -> ! {
    panic!("Attempted to access dropped Scoped")
}
