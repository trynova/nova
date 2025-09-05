// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        scripts_and_modules::module::module_semantics::abstract_module_records::AbstractModule,
        types::String,
    },
    engine::context::{Bindable, bindable_handle},
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};

use super::Module;

#[derive(Debug, Clone)]
pub struct ModuleHeapData<'a> {
    pub(super) module: AbstractModule<'a>,
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

bindable_handle!(ModuleHeapData);

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
