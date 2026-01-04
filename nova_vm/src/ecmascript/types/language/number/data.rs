// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Deref, DerefMut};

use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct NumberHeapData {
    pub(crate) data: f64,
}

impl From<f64> for NumberHeapData {
    #[inline(always)]
    fn from(data: f64) -> Self {
        Self { data }
    }
}

impl From<NumberHeapData> for f64 {
    fn from(value: NumberHeapData) -> Self {
        value.data
    }
}

impl Deref for NumberHeapData {
    type Target = f64;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for NumberHeapData {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl HeapMarkAndSweep for NumberHeapData {
    fn mark_values(&self, _queues: &mut WorkQueues) {
        let Self { data: _ } = self;
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        let Self { data: _ } = self;
    }
}
