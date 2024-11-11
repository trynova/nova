// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::RegExpFlags;

use crate::{
    ecmascript::types::{OrdinaryObject, String},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct RegExpHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    // _regex: RegExp,
    pub(crate) original_source: String,
    pub(crate) original_flags: RegExpFlags,
}

impl Default for RegExpHeapData {
    fn default() -> Self {
        Self {
            object_index: Default::default(),
            original_source: String::EMPTY_STRING,
            original_flags: RegExpFlags::empty(),
        }
    }
}

impl HeapMarkAndSweep for RegExpHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            original_source,
            original_flags: _,
        } = self;
        object_index.mark_values(queues);
        original_source.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            original_source,
            original_flags: _,
        } = self;
        object_index.sweep_values(compactions);
        original_source.sweep_values(compactions);
    }
}
