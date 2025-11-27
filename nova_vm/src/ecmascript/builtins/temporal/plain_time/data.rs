// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::NoGcScope;
use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};
use crate::{
    ecmascript::types::OrdinaryObject,
    engine::context::{bindable_handle, trivially_bindable},
};

#[derive(Debug, Clone, Copy)]
pub struct PlainTimeHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) plain_time: temporal_rs::PlainTime,
}

impl PlainTimeHeapData<'_> {
    pub fn default() -> Self {
        Self {
            object_index: None,
            plain_time: temporal_rs::PlainTime::new(0, 0, 0, 0, 0, 0).unwrap(),
        }
    }
}

trivially_bindable!(temporal_rs::PlainTime);
bindable_handle!(PlainTimeHeapData);

impl HeapMarkAndSweep for PlainTimeHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            plain_time: _,
        } = self;

        object_index.mark_values(queues);
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            plain_time: _,
        } = self;

        object_index.sweep_values(compactions);
    }
}
