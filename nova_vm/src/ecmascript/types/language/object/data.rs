// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{Object, PropertyKey};
use crate::{
    ecmascript::{execution::Agent, types::Value},
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues, element_array::PropertyStorageVector},
};

#[derive(Debug, Clone, Copy)]
pub struct ObjectHeapData<'a> {
    // TODO: move prototype, key index, cap, extensible, and length into
    // shapes (#647). What remains here would be just shape and values indexes?
    // That would bring object side down to just 8 bytes, which is pretty
    // acceptable.
    // Possible variations on the theme:
    // - Shape index u32 gives 1 bit to extensible boolean; shape itself
    //   doesn't know if it is extensible or not.
    // - Shape index u32 gives 5-6 bits to cap: capacity of the keys & values
    //   is kept in the index value, saving memory in shape and helping
    //   indexing at the cost of a lower maximum shape count.
    pub prototype: Option<Object<'a>>,
    pub property_storage: PropertyStorageVector<'a>,
}

impl<'a> ObjectHeapData<'a> {
    pub fn new(prototype: Value, property_storage: PropertyStorageVector<'a>) -> Self {
        let prototype = if prototype.is_null() {
            None
        } else {
            // TODO: Throw error.
            Some(Object::try_from(prototype.unbind()).unwrap())
        };
        Self {
            prototype,
            property_storage,
        }
    }

    pub fn has(&self, agent: &Agent, key: PropertyKey) -> bool {
        agent.heap.elements.has(&self.property_storage, key)
    }

    pub fn is_empty(&self) -> bool {
        self.property_storage.len() == 0
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
            prototype,
            property_storage,
        } = self;
        prototype.mark_values(queues);
        property_storage.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            prototype,
            property_storage,
        } = self;
        prototype.sweep_values(compactions);
        property_storage.sweep_values(compactions);
    }
}
