// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            Array, promise::Promise,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::Agent,
        types::{IntoValue, Value},
    },
    engine::context::{Bindable, GcScope, NoGcScope, bindable_handle},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

#[derive(Debug, Clone, Copy)]
pub struct PromiseAllRecord<'a> {
    pub(crate) remaining_elements_count: u32,
    pub(crate) result_array: Array<'a>,
    pub(crate) promise: Promise<'a>,
}

impl<'a> PromiseAllRecord<'a> {
    pub(crate) fn on_promise_fulfilled(
        self,
        agent: &mut Agent,
        index: u32,
        value: Value<'a>,
        gc: GcScope<'a, '_>,
    ) {
        let value = value.bind(gc.nogc());

        let elements = self.result_array.as_mut_slice(agent);
        elements[index as usize] = Some(value.unbind());

        //ii. If remainingElementsCount.[[Value]] = 0, then
        if self.remaining_elements_count == 0 {
            // 1. Let valuesArray be CreateArrayFromList(values).
            // 2. Perform ? Call(resultCapability.[[Resolve]], undefined, « valuesArray »).
            let capability = PromiseCapability::from_promise(self.promise, true);
            capability.resolve(agent, self.result_array.into_value().unbind(), gc);
        }
    }

    pub(crate) fn on_promise_rejected(
        self,
        agent: &mut Agent,
        value: Value<'a>,
        gc: NoGcScope<'a, '_>,
    ) {
        let value = value.bind(gc);
        let capability = PromiseCapability::from_promise(self.promise, true);
        capability.reject(agent, value.unbind(), gc);
    }
}

impl HeapMarkAndSweep for PromiseAllRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            remaining_elements_count: _,
            result_array,
            promise,
        } = self;
        result_array.mark_values(queues);
        promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            remaining_elements_count: _,
            result_array,
            promise,
        } = self;
        result_array.sweep_values(compactions);
        promise.sweep_values(compactions);
    }
}

bindable_handle!(PromiseAllRecord);
