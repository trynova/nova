// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::agent::ExceptionType,
        types::{OrdinaryObject, String, Value},
    },
    engine::context::bindable_handle,
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ErrorHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) kind: ExceptionType,
    pub(crate) message: Option<String<'a>>,
    pub(crate) cause: Option<Value<'a>>,
    // TODO: stack? name?
}

impl<'a> ErrorHeapData<'a> {
    pub(crate) fn new(
        kind: ExceptionType,
        message: Option<String<'a>>,
        cause: Option<Value<'a>>,
    ) -> Self {
        Self {
            object_index: None,
            kind,
            message,
            cause,
        }
    }
}

bindable_handle!(ErrorHeapData);

impl HeapMarkAndSweep for ErrorHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            kind: _,
            message,
            cause,
        } = self;

        object_index.mark_values(queues);
        message.mark_values(queues);
        cause.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            kind: _,
            message,
            cause,
        } = self;
        object_index.sweep_values(compactions);
        message.sweep_values(compactions);
        cause.sweep_values(compactions);
    }
}
