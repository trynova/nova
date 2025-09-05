// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::String,
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct SymbolHeapData<'a> {
    pub(crate) descriptor: Option<String<'a>>,
}

bindable_handle!(SymbolHeapData);

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
