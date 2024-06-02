//! ## [27.2.1.1 PromiseCapability Records]()

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::call_function,
        builtins::ArgumentsList,
        execution::{agent::JsError, Agent, JsResult},
        types::{Function, Object, Value},
    },
    heap::{
        indexes::BaseIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct PromiseCapabilityRecord {
    /// \[\[Promise\]\]
    ///
    /// an Object
    ///
    /// An object that is usable as a promise.
    pub(crate) promise: Object,
    /// \[\[Resolve\]\]
    ///
    /// a function object
    ///
    /// The function that is used to resolve the given promise.
    pub(crate) resolve: Function,
    /// \[\[Reject\]\]
    ///
    /// a function object
    ///
    /// The function that is used to reject the given promise.
    pub(crate) reject: Function,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct PromiseCapability(BaseIndex<PromiseCapabilityRecord>);

impl PromiseCapability {
    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl HeapMarkAndSweep for PromiseCapability {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_capability_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .promise_capability_records
            .shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for PromiseCapabilityRecord {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.promise.mark_values(queues);
        self.reject.mark_values(queues);
        self.resolve.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.promise.sweep_values(compactions);
        self.reject.sweep_values(compactions);
        self.resolve.sweep_values(compactions);
    }
}

impl Index<PromiseCapability> for Agent {
    type Output = PromiseCapabilityRecord;

    fn index(&self, index: PromiseCapability) -> &Self::Output {
        self.heap
            .promise_capability_records
            .get(index.get_index())
            .expect("PromiseCapability out of bounds")
            .as_ref()
            .expect("PromiseCapability slot empty")
    }
}

impl IndexMut<PromiseCapability> for Agent {
    fn index_mut(&mut self, index: PromiseCapability) -> &mut Self::Output {
        self.heap
            .promise_capability_records
            .get_mut(index.get_index())
            .expect("PromiseCapability out of bounds")
            .as_mut()
            .expect("PromiseCapability slot empty")
    }
}

impl CreateHeapData<PromiseCapabilityRecord, PromiseCapability> for Heap {
    fn create(&mut self, data: PromiseCapabilityRecord) -> PromiseCapability {
        self.promise_capability_records.push(Some(data));
        PromiseCapability(BaseIndex::last(&self.promise_capability_records))
    }
}

/// ### [27.2.1.1.1 IfAbruptRejectPromise ( value, capability )](https://tc39.es/ecma262/#sec-ifabruptrejectpromise)
///
/// IfAbruptRejectPromise is a shorthand for a sequence of algorithm steps that
/// use a PromiseCapability Record. An algorithm step of the form:
///
/// ```
/// 1. IfAbruptRejectPromise(value, capability).
/// ```
///
/// means the same thing as:
/// ```
/// 1. Assert: value is a Completion Record.
/// 2. If value is an abrupt completion, then
///     a. Perform ? Call(capability.[[Reject]], undefined, « value.[[Value]] »).
///     b. Return capability.[[Promise]].
/// 3. Else,
///     a. Set value to ! value.
/// ```
#[inline(always)]
pub(crate) fn if_abrupt_reject_promise<T>(
    agent: &mut Agent,
    value: JsResult<T>,
    capability: PromiseCapability,
) -> JsResult<T> {
    value.or_else(|err| {
        // If abrupt completion, call reject and make caller return the
        // capability promise
        let PromiseCapabilityRecord {
            promise, reject, ..
        } = agent[capability];
        call_function(
            agent,
            reject,
            Value::Undefined,
            Some(ArgumentsList(&[err.value()])),
        )?;
        // Note: We return an error here so that caller gets to call this
        // function with the ? operator
        Err(JsError::new(promise.into_value()))
    })
}
