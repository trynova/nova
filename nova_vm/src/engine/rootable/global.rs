use std::marker::PhantomData;

use crate::{
    ecmascript::execution::Agent,
    engine::rootable::{HeapRootRef, Rootable},
};

/// # Global heap root
///
/// This type roots a heap-allocated JavaScript engine value until explicitly
/// released. A rooted value cannot be garbage collected, so accessing the
/// rooted value is always safe. The Global can be thought of as a unique
/// pointer to a heap allocation in system programming languages. As long as
/// the pointer lives, the memory on the heap will not be released.
#[derive(Debug, PartialEq)]
pub struct Global<T: Rootable>(T::RootRepr, PhantomData<T>);

impl<T: Rootable> Global<T> {
    /// Root the given value into a Global, keeping it from being garbage
    /// collected until the Global is explicitly released.
    #[must_use]
    pub fn new(agent: &Agent, value: T) -> Self {
        let value = match T::to_root_repr(value) {
            Ok(stack_repr) => {
                // The value doesn't need rooting.
                return Self(stack_repr, PhantomData);
            }
            Err(heap_data) => heap_data,
        };
        let mut globals = agent.heap.globals.borrow_mut();
        let reused_index = globals.iter_mut().enumerate().find_map(|(index, entry)| {
            if entry.is_none() {
                *entry = Some(value);
                Some(index)
            } else {
                None
            }
        });
        let heap_ref = if let Some(reused_index) = reused_index {
            HeapRootRef::from_index(reused_index)
        } else {
            let next_index = globals.len();
            globals.push(Some(value));
            HeapRootRef::from_index(next_index)
        };
        Self(T::from_heap_ref(heap_ref), Default::default())
    }

    /// Take the rooted value from inside this Global, releasing it in the
    /// process. Using the Global is not possible after this call.
    pub fn take(self, agent: &Agent) -> T {
        let heap_ref = match T::from_root_repr(&self.0) {
            Ok(value) => {
                // The value didn't need rooting
                return value;
            }
            Err(heap_ref) => heap_ref,
        };
        // Leave a `None` in the index and return the value
        let heap_data = agent
            .heap
            .globals
            .borrow_mut()
            .get_mut(heap_ref.to_index())
            .unwrap()
            .take()
            .unwrap();
        let Some(value) = T::from_heap_data(heap_data) else {
            panic!("Invalid Global returned different type than expected");
        };
        value
    }

    /// Access the rooted value from inside this Global without releasing the
    /// Global.
    pub fn get(&self, agent: &mut Agent) -> T {
        let heap_ref = match T::from_root_repr(&self.0) {
            Ok(value) => {
                // The value didn't need rooting
                return value;
            }
            Err(heap_ref) => heap_ref,
        };
        let heap_data = *agent
            .heap
            .globals
            .borrow_mut()
            .get_mut(heap_ref.to_index())
            .unwrap()
            .as_ref()
            .unwrap();
        let Some(value) = T::from_heap_data(heap_data) else {
            panic!("Invalid Global returned different type than expected");
        };
        value
    }

    /// Create a clone of this Global. Cloning a global means that both the
    /// original Global and the cloned one must be explicitly released before
    /// the rooted value can be garbage collected.
    #[must_use]
    pub fn clone(&self, agent: &mut Agent) -> Self {
        let value = self.get(agent);
        Self::new(agent, value)
    }
}
