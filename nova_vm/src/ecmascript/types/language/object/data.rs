use super::Object;
use crate::{
    ecmascript::{execution::Agent, types::Value},
    heap::element_array::ElementsVector,
};

#[derive(Debug, Clone, Copy)]
pub struct ObjectHeapData {
    pub extensible: bool,
    pub prototype: Option<Object>,
    pub keys: ElementsVector,
    pub values: ElementsVector,
}

impl ObjectHeapData {
    pub fn new(
        extensible: bool,
        prototype: Value,
        keys: ElementsVector,
        values: ElementsVector,
    ) -> Self {
        let prototype = if prototype.is_null() {
            None
        } else {
            // TODO: Throw error.
            Some(Object::try_from(prototype).unwrap())
        };
        Self {
            extensible,
            // TODO: Number, Boolean, etc. objects exist. These can all be
            // modeled with their own heap vector or alternatively by adding
            // a [[PrimitiveValue]] field to objects: Normally this field is None
            // to signal that the object is its own primitive value. For
            // Number objects etc the field is Some(Value).
            // TODO: Move prototype and key vector into shapes
            prototype,
            keys,
            values,
        }
    }

    pub fn has(&self, agent: &Agent, key: Value) -> bool {
        debug_assert!(key.is_string() || key.is_number() || key.is_symbol());
        agent.heap.elements.has(self.keys, key)
    }
}
