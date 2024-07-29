// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Object;
use crate::{
    ecmascript::{execution::Agent, types::Value},
    heap::{element_array::ElementsVector, CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct ObjectHeapData {
    pub extensible: bool,
    pub prototype: Option<Object<'gen>>,
    pub keys: ElementsVector,
    pub values: ElementsVector,
}

impl ObjectHeapData {
    pub fn new(
        extensible: bool,
        prototype: Value<'gen>,
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
            // TODO: Number<'gen>, Boolean, etc. objects exist. These can all be
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

    pub fn has(&self, agent: &Agent<'gen>, key: Value<'gen>) -> bool {
        debug_assert!(key.is_string() || key.is_number() || key.is_symbol());
        agent.heap.elements.has(self.keys, key)
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for ObjectHeapData {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.keys.mark_values(queues);
        self.values.mark_values(queues);
        self.prototype.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.keys.sweep_values(compactions);
        self.values.sweep_values(compactions);
        self.prototype.sweep_values(compactions);
    }
}
