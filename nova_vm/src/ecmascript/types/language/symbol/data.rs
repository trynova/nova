// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::String,
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData<'a> {
    pub(crate) descriptor: Option<String<'a>>,
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SymbolHeapData<'_> {
    type Of<'a> = SymbolHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for SymbolHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { descriptor } = self;
        descriptor.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { descriptor } = self;
        descriptor.sweep_values(compactions);
    }
}
