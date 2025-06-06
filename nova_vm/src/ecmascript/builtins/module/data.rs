// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use small_string::SmallString;

use crate::{
    ecmascript::types::{HeapString, String},
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::Module;

#[derive(Debug, Clone)]
pub struct ModuleHeapData<'a> {
    pub(crate) exports: Box<[String<'a>]>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolvedBindingName {
    String(HeapString<'static>),
    SmallString(SmallString),
    Namespace,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedBinding {
    /// \[\[Module]]
    pub(super) module: Option<Module<'static>>,
    /// \[\[BindingName]]
    pub(super) binding_name: ResolvedBindingName,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolveExportResult {
    Ambiguous,
    Resolved(ResolvedBinding),
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ModuleHeapData<'_> {
    type Of<'a> = ModuleHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for ModuleHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { exports } = self;
        for ele in exports.iter() {
            ele.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { exports } = self;
        for ele in exports.iter_mut() {
            ele.sweep_values(compactions);
        }
    }
}
