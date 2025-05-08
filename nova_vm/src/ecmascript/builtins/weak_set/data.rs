// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashSet;

use crate::{
    ecmascript::{execution::WeakKey, types::OrdinaryObject},
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues, sweep_side_set},
};

#[derive(Debug, Default)]
pub struct WeakSetHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    /// ### \[\[WeakSetData]]
    weak_set_data: AHashSet<WeakKey<'a>>,
}

impl WeakSetHeapData<'_> {
    /// Add a weakly holdable to the WeakSet.
    pub(crate) fn add(&mut self, value: WeakKey) {
        self.weak_set_data.insert(value.unbind());
    }

    /// Remove a weakly holdable from the WeakSet.
    pub(crate) fn delete(&mut self, value: WeakKey) -> bool {
        self.weak_set_data.remove(&value.unbind())
    }

    /// Returns true if the WeakSet contains the given weakly holdable key.
    pub(crate) fn has(&mut self, value: WeakKey) -> bool {
        self.weak_set_data.contains(&value.unbind())
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for WeakSetHeapData<'_> {
    type Of<'a> = WeakSetHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for WeakSetHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            // Note: WeakSet data is never marked on its own; that's its whole
            // point.
            weak_set_data: _,
        } = self;
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            weak_set_data,
        } = self;
        object_index.sweep_values(compactions);
        sweep_side_set(weak_set_data, compactions);
    }
}
