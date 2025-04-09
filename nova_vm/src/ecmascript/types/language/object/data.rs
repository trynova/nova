// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Object;
use crate::{
    ecmascript::{execution::Agent, types::Value},
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues, element_array::ElementsVector},
};

#[derive(Debug, Clone, Copy)]
pub struct ObjectHeapData<'a> {
    pub extensible: bool,
    pub prototype: Option<Object<'a>>,
    pub keys: ElementsVector<'a>,
    pub values: ElementsVector<'a>,
}

impl<'a> ObjectHeapData<'a> {
    pub fn new(
        extensible: bool,
        prototype: Value,
        keys: ElementsVector<'a>,
        values: ElementsVector<'a>,
    ) -> Self {
        let prototype = if prototype.is_null() {
            None
        } else {
            // TODO: Throw error.
            Some(Object::try_from(prototype.unbind()).unwrap())
        };
        Self {
            extensible,
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ObjectHeapData<'_> {
    type Of<'a> = ObjectHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for ObjectHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            extensible: _,
            prototype,
            keys,
            values,
        } = self;

        keys.mark_values(queues);
        values.mark_values(queues);
        prototype.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            extensible: _,
            prototype,
            keys,
            values,
        } = self;
        keys.sweep_values(compactions);
        values.sweep_values(compactions);
        prototype.sweep_values(compactions);
    }
}
