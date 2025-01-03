// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::Object,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone)]
pub struct ProxyHeapData {
    /// [[ProxyTarget]]
    pub(crate) proxy_target: Option<Object>,
    /// [[ProxyHandler]]
    pub(crate) proxy_handler: Option<Object>,
}

impl HeapMarkAndSweep for ProxyHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.proxy_target.mark_values(queues);
        self.proxy_handler.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.proxy_target.sweep_values(compactions);
        self.proxy_handler.sweep_values(compactions);
    }
}
