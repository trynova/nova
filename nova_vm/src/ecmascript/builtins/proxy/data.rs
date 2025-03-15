// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::Object,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone)]
pub enum ProxyHeapData {
    /// Proxy has not been revoked.
    NonRevoked {
        /// [[ProxyTarget]]
        proxy_target: Object<'static>,
        /// [[ProxyHandler]]
        proxy_handler: Object<'static>,
    },
    /// A callable Proxy was revoked.
    RevokedCallable,
    /// A non-callable Proxy was revoked.
    Revoked,
}

impl HeapMarkAndSweep for ProxyHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self::NonRevoked {
            proxy_target,
            proxy_handler,
        } = self
        else {
            return;
        };
        proxy_target.mark_values(queues);
        proxy_handler.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self::NonRevoked {
            proxy_target,
            proxy_handler,
        } = self
        else {
            return;
        };
        proxy_target.sweep_values(compactions);
        proxy_handler.sweep_values(compactions);
    }
}
