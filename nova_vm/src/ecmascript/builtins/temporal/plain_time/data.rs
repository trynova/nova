// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::OrdinaryObject,
    engine::{NoGcScope, bindable_handle, trivially_bindable},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy, Default)]
pub struct PlainTimeRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) plain_time: temporal_rs::PlainTime,
}

impl PlainTimeRecord<'_> {
    pub fn default() -> Self {
        Self {
            object_index: None,
            plain_time: temporal_rs::PlainTime::try_new(0, 0, 0, 0, 0, 0).unwrap(),
        }
    }
}

trivially_bindable!(temporal_rs::PlainTime);
bindable_handle!(PlainTimeRecord);

impl HeapMarkAndSweep for PlainTimeRecord<'static> {
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
