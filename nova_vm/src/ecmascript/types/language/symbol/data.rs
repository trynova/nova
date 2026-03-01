// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::String,
    engine::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SymbolHeapData<'a> {
    /// \[\[Description]]
    pub(super) description: Option<String<'a>>,
}

impl<'a> SymbolHeapData<'a> {
    pub(crate) fn new(description: String<'a>) -> Self {
        Self {
            description: Some(description),
        }
    }
}

bindable_handle!(SymbolHeapData);

impl HeapMarkAndSweep for SymbolHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { description } = self;
        description.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { description } = self;
        description.sweep_values(compactions);
    }
}
