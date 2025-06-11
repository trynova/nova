// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        scripts_and_modules::module::module_semantics::source_text_module_records::SourceTextModule,
        types::String,
    },
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};

use super::Module;

#[derive(Debug, Clone)]
pub struct ModuleHeapData<'a> {
    pub(super) module: SourceTextModule<'a>,
    pub(super) exports: Box<[String<'a>]>,
}

impl<'a> CreateHeapData<ModuleHeapData<'a>, Module<'a>> for Heap {
    fn create(&mut self, data: ModuleHeapData<'a>) -> Module<'a> {
        let index = self.modules.len();
        self.modules.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<ModuleHeapData<'static>>>();
        Module::from_index(index)
    }
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
        let Self { module, exports } = self;
        module.mark_values(queues);
        for ele in exports.iter() {
            ele.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { module, exports } = self;
        module.sweep_values(compactions);
        for ele in exports.iter_mut() {
            ele.sweep_values(compactions);
        }
    }
}
