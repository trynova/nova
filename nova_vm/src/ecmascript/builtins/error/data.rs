// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::agent::ExceptionType,
        types::{OrdinaryObject, String, Value},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct ErrorHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    pub(crate) kind: ExceptionType,
    pub(crate) message: Option<String<'gen>>,
    pub(crate) cause: Option<Value<'gen>>,
    // TODO: stack? name?
}

impl<'gen> ErrorHeapData<'gen> {
    pub(crate) fn new(kind: ExceptionType, message: Option<String<'gen>>, cause: Option<Value<'gen>>) -> Self {
        Self {
            object_index: None,
            kind,
            message,
            cause,
        }
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for ErrorHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.object_index.mark_values(queues);
        self.message.mark_values(queues);
        self.cause.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.message.sweep_values(compactions);
        self.cause.sweep_values(compactions);
    }
}
