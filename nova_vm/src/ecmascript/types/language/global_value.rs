use std::marker::PhantomData;

use crate::ecmascript::execution::Agent;

use super::{IntoValue, Value};

/// Stores a Value on the Agent heap as a rooted Value.
///
/// A rooted Value cannot be garbage collected. It is safe to thus get the
/// Value out of a Global at any time. The Global can be thought of as a
/// unique pointer to a heap allocation in system programming languages. As
/// long as the pointer lives, the memory on the heap will not be released.
#[derive(Debug, PartialEq)]
pub struct Global<'gen, T>(u32, PhantomData<T>) where T: IntoValue<'gen> + TryFrom<Value<'gen>>;

impl<'gen, T: IntoValue<'gen> + TryFrom<Value<'gen>>> Global<'gen, T> {
    /// Register a value as global.
    #[must_use]
    pub fn new(agent: &mut Agent<'gen>, value: T) -> Self {
        let reused_index = agent
            .heap
            .globals
            .iter_mut()
            .enumerate()
            .find_map(|(index, entry)| {
                if entry.is_none() {
                    *entry = Some(value.into_value());
                    let index = u32::try_from(index).expect("Globals overflowed");
                    Some(index)
                } else {
                    None
                }
            });
        if let Some(reused_index) = reused_index {
            Global(reused_index, Default::default())
        } else {
            let next_index = agent.heap.globals.len();
            let next_index = u32::try_from(next_index).expect("Globals overflowed");
            agent.heap.globals.push(Some(value.into_value()));
            Global(next_index, Default::default())
        }
    }

    /// Unregister this global value.
    pub fn take(self, agent: &mut Agent<'gen>) -> T {
        // Leave a `None` in the index and return the value
        let value = agent
            .heap
            .globals
            .get_mut(self.0 as usize)
            .unwrap()
            .take()
            .unwrap();
        let Ok(value) = T::try_from(value) else {
            panic!("Invalid Global returned different type than expected");
        };
        value
    }

    pub fn get(&self, agent: &mut Agent<'gen>) -> T {
        let value = *agent
            .heap
            .globals
            .get_mut(self.0 as usize)
            .unwrap()
            .as_ref()
            .unwrap();
        let Ok(value) = T::try_from(value) else {
            panic!("Invalid Global returned different type than expected");
        };
        value
    }

    #[must_use]
    pub fn clone(&self, agent: &mut Agent<'gen>) -> Self {
        let value = self.get(agent);
        Self::new(agent, value)
    }
}
