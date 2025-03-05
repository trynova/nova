// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::marker::PhantomData;

use crate::{
    ecmascript::execution::Agent,
    engine::{
        context::{Bindable, NoGcScope},
        rootable::{HeapRootRef, Rootable},
    },
};

/// # Scoped heap root
///
/// This type roots a heap-allocated JavaScript engine value for the duration
/// of the current JavaScript call context, roughly corresponding to a native
/// call scope. Stack-allocated values avoid rooting. Rooted values cannot be
/// garbage collected, so accessing the rooted value is always safe within the
/// current call context. This type is intended for cheap rooting of JavaScript
/// Values that need to be used after calling into functions that may trigger
/// garbage collection.
#[derive(Debug, Hash, Clone)]
#[repr(transparent)]
pub struct Scoped<'a, T: 'static + Rootable> {
    pub(crate) inner: T::RootRepr,
    _marker: PhantomData<T>,
    _scope: PhantomData<&'a ()>,
}

impl<T: 'static + Rootable> Scoped<'static, T> {
    #[inline(always)]
    pub const fn from_root_repr(value: T::RootRepr) -> Scoped<'static, T> {
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
    pub unsafe fn take(self, agent: &Agent) -> T {
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
                *self = Self {
                    inner: stack_repr,
                    _marker: PhantomData,
                    _scope: PhantomData,
                };
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

    pub fn from_scoped<U: 'static + Rootable>(
        agent: &Agent,
        scoped: Scoped<'scope, U>,
        value: T,
        gc: NoGcScope<'_, 'scope>,
    ) -> Self {
        let heap_data = match T::to_root_repr(value) {
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
        let Err(heap_root_ref) = U::from_root_repr(&scoped.inner) else {
            // Previous Scoped is an on-stack value, we can't reuse its heap
            // slot.
            return Self::new(agent, value, gc);
        };
        // Previous Scoped had a heap slot, we can reuse it.
        let mut stack_refs_borrow = agent.stack_refs.borrow_mut();
        let Some(heap_slot) = stack_refs_borrow.get_mut(heap_root_ref.to_index()) else {
            handle_bound_check_failure()
        };
        *heap_slot = heap_data;
        Self {
            inner: T::from_heap_ref(heap_root_ref),
            _marker: PhantomData,
            _scope: PhantomData,
        }
    }
}

#[cold]
#[inline(never)]
fn handle_invalid_scoped_conversion() -> ! {
    panic!("Attempted to convert mismatched Scoped");
}

#[cold]
#[inline(never)]
fn handle_index_overflow() -> ! {
    panic!("Scoped stack overflowed");
}

#[cold]
#[inline(never)]
fn handle_bound_check_failure() -> ! {
    panic!("Attempted to access dropped Scoped")
}
