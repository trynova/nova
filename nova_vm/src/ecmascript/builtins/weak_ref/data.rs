// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{execution::WeakKey, types::OrdinaryObject},
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Default, Debug, Clone)]
pub struct WeakRefHeapData<'a> {
    pub(super) object_index: Option<OrdinaryObject<'a>>,
    /// ### \[\[WeakRefTarget]]
    pub(super) weak_ref_target: Option<WeakKey<'a>>,
    /// ### \[\[KeptAlive]]
    ///
    /// Instead of storing a list of kept-alive targets in Agents, we keep only
    /// a boolean there and clear all kept_alive booleans at the end of a job
    /// run.
    pub(super) kept_alive: bool,
}

impl WeakRefHeapData<'_> {
    pub(crate) fn clear_kept_objects(&mut self) {
        self.kept_alive = false;
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for WeakRefHeapData<'_> {
    type Of<'a> = WeakRefHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for WeakRefHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            weak_ref_target: value,
            kept_alive: is_strong,
        } = self;
        object_index.mark_values(queues);
        if *is_strong {
            value.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            weak_ref_target: value,
            kept_alive: _,
        } = self;
        object_index.sweep_values(compactions);
        value.sweep_values(compactions);
    }
}
