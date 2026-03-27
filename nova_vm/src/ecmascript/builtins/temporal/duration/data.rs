// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::OrdinaryObject,
    engine::{NoGcScope, bindable_handle, trivially_bindable},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DurationRecord<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) duration: temporal_rs::Duration,
}

trivially_bindable!(temporal_rs::Duration);
trivially_bindable!(temporal_rs::partial::PartialDuration);
bindable_handle!(DurationRecord);

impl HeapMarkAndSweep for DurationRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            duration: _,
        } = self;

        object_index.mark_values(queues);
    }
    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            duration: _,
        } = self;

        object_index.sweep_values(compactions);
    }
}
