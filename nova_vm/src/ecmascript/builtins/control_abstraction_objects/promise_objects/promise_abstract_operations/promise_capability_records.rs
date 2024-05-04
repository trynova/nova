//! ## [27.2.1.1 PromiseCapability Records]()

use std::ops::{Index, IndexMut};

use crate::{ecmascript::{abstract_operations::operations_on_objects::{call, call_function}, builtins::ArgumentsList, execution::{agent::JsError, Agent, JsResult}, types::{Function, IntoValue, Object, Value}}, heap::{indexes::BaseIndex, Heap}};


#[derive(Debug, Clone, Copy)]
pub(crate) struct PromiseCapabilityRecord {
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

pub(crate) type PromiseCapability = BaseIndex<PromiseCapabilityRecord>;

impl Index<PromiseCapability> for Agent {
    type Output = PromiseCapabilityRecord;

    fn index(&self, index: PromiseCapability) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<PromiseCapability> for Agent {
    fn index_mut(&mut self, index: PromiseCapability) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<PromiseCapability> for Heap {
    type Output = PromiseCapabilityRecord;

    fn index(&self, index: PromiseCapability) -> &Self::Output {
        self.promise_capability_records
            .get(index.into_index())
            .expect("PromiseCapability out of bounds")
            .as_ref()
            .expect("PromiseCapability slot empty")
    }
}

impl IndexMut<PromiseCapability> for Heap {
    fn index_mut(&mut self, index: PromiseCapability) -> &mut Self::Output {
        self.promise_capability_records
            .get_mut(index.into_index())
            .expect("PromiseCapability out of bounds")
            .as_mut()
            .expect("PromiseCapability slot empty")
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
pub(crate) fn if_abrupt_reject_promise<T>(agent: &mut Agent, value: JsResult<T>, capability: PromiseCapability) -> JsResult<T> {
    value.or_else(|err| {
        // If abrupt completion, call reject and make caller return the
        // capability promise
        let PromiseCapabilityRecord { promise,  reject, .. } = agent[capability];
        call_function(agent, reject, Value::Undefined, Some(ArgumentsList(&[err.0])))?;
        // Note: We return an error here so that caller gets to call this
        // function with the ? operator
        Err(JsError(promise.into_value()))
    })
}