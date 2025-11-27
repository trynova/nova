// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::engine::context::NoGcScope;
use crate::{
    ecmascript::types::OrdinaryObject,
    engine::context::{bindable_handle, trivially_bindable},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct DurationHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) duration: temporal_rs::Duration,
}

impl DurationHeapData<'_> {
    pub fn default() -> Self {
        todo!()
    }
}

trivially_bindable!(temporal_rs::Duration);
bindable_handle!(DurationHeapData);

impl HeapMarkAndSweep for DurationHeapData<'static> {
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
