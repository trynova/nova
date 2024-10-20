// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::marker::PhantomData;

use crate::{
    ecmascript::execution::Agent,
    engine::rootable::{HeapRootRef, Rootable},
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
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Scoped<T: Rootable> {
    inner: T::RootRepr,
    _marker: PhantomData<T>,
}

impl<T: Rootable> Scoped<T> {
    pub fn new(agent: &Agent, value: T) -> Self {
        let value = match T::to_root_repr(value) {
            Ok(stack_repr) => {
                // The value doesn't need rooting.
                return Self {
                    inner: stack_repr,
                    _marker: PhantomData,
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
        }
    }

    pub fn get(self, agent: &Agent) -> T {
        match T::from_root_repr(&self.inner) {
            Ok(value) => value,
            Err(heap_root_ref) => {
                let Some(&heap_data) = agent.stack_refs.borrow().get(heap_root_ref.to_index())
                else {
                    handle_bound_check_failure()
                };
                let Some(value) = T::from_heap_data(heap_data) else {
                    handle_invalid_local_conversion()
                };
                value
            }
        }
    }
}

#[cold]
#[inline(never)]
fn handle_invalid_local_conversion() -> ! {
    panic!("Attempted to convert mismatched Local");
}

#[cold]
#[inline(never)]
fn handle_index_overflow() -> ! {
    panic!("Locals stack overflowed");
}

#[cold]
#[inline(never)]
fn handle_bound_check_failure() -> ! {
    panic!("Attempted to access dropped Local")
}
