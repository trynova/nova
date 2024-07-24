use crate::ecmascript::execution::Agent;

use super::Value;

/// Holds the position of a global value in the heap.
/// The main advantage of this is that the garbage collector will not move the elements
/// of the global values vector, making it possible to maintain a position of any value across
/// multiple calls of the garbage collector.
///
/// This might be useful to resolve promises asynchronously for example.
pub struct GlobalValue(usize);

impl GlobalValue {
    /// Register a value as global.
    pub fn new(agent: &mut Agent, value: impl Into<Value>) -> Self {
        let available_index = Self::find_available_index(agent);
        agent
            .heap
            .globals
            .insert(available_index, Some(value.into()));
        Self(available_index)
    }

    /// Unregister this global value.
    #[must_use]
    pub fn take(&self, agent: &mut Agent) -> Value {
        // Leave a `None` in the index and return the value
        agent.heap.globals.get_mut(self.0).unwrap().take().unwrap()
    }

    /// Find an available index in the global values vector.
    fn find_available_index(agent: &mut Agent) -> usize {
        let mut available_index = 0;

        loop {
            // Index has been freed previously
            if let Some(None) = agent.heap.globals.get(available_index) {
                break;
            }

            // Global values vector is full, the capacity must increase
            if available_index == agent.heap.globals.len() {
                agent.heap.globals.push(None);
                available_index = agent.heap.globals.len() - 1;
                break;
            }

            // Advance the index
            available_index += 1;
        }

        available_index
    }
}
