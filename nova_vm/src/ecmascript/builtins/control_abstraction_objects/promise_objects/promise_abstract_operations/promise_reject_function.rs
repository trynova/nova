use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::promise::Promise,
        execution::Agent,
        types::{
            Function, InternalMethods, IntoFunction, IntoObject, IntoValue, Object, OrdinaryObject,
            OrdinaryObjectInternalSlots, Value,
        },
    },
    heap::{indexes::BaseIndex, CreateHeapData, Heap, HeapMarkAndSweep},
};

/// ### [27.2.1.3.1 Promise Reject Functions]()
///
/// A promise reject function is an anonymous built-in function that has
/// \[\[Promise\]\] and \[\[AlreadyResolved\]\] internal slots.
///
/// The "length" property of a promise reject function is 1𝔽.
#[derive(Debug, Clone, Copy)]
pub struct PromiseRejectFunctionHeapData {
    /// \[\[Promise\]\]
    pub(crate) promise: Promise,
    /// \[\[AlreadyResolved\]\]
    pub(crate) already_resolved: bool,
    pub(crate) object_index: Option<OrdinaryObject>,
}

pub(crate) type BuiltinPromiseRejectFunctionIndex = BaseIndex<PromiseRejectFunctionHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinPromiseRejectFunction(pub(crate) BuiltinPromiseRejectFunctionIndex);

impl BuiltinPromiseRejectFunction {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<BuiltinPromiseRejectFunction> for Function {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value)
    }
}

impl IntoFunction for BuiltinPromiseRejectFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<BuiltinPromiseRejectFunction> for Object {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value)
    }
}

impl IntoObject for BuiltinPromiseRejectFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<BuiltinPromiseRejectFunction> for Value {
    fn from(value: BuiltinPromiseRejectFunction) -> Self {
        Self::BuiltinPromiseRejectFunction(value)
    }
}

impl IntoValue for BuiltinPromiseRejectFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl PromiseRejectFunctionHeapData {
    /// When a promise reject function is called with argument reason, the
    /// following steps are taken:
    pub(crate) fn call(agent: &mut Agent, _reason: Value) {
        // 1. Let F be the active function object.
        let f = agent.running_execution_context().function.unwrap();
        // 2. Assert: F has a [[Promise]] internal slot whose value is an Object.
        let Function::BuiltinPromiseRejectFunction(f) = f else {
            unreachable!();
        };
        // 3. Let promise be F.[[Promise]].
        // 4. Let alreadyResolved be F.[[AlreadyResolved]].
        let PromiseRejectFunctionHeapData {
            already_resolved, ..
        } = agent[f];
        // 5. If alreadyResolved.[[Value]] is true, return undefined.
        if !already_resolved {
            // 6. Set alreadyResolved.[[Value]] to true.
            agent[f].already_resolved = true;
            // 7. Perform RejectPromise(promise, reason).
            // reject_promise(agent, promise, reason);
            // 8. Return undefined.
        }
    }
}

impl OrdinaryObjectInternalSlots for BuiltinPromiseRejectFunction {
    fn internal_extensible(self, _agent: &Agent) -> bool {
        todo!()
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!()
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        todo!()
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!()
    }
}

impl InternalMethods for BuiltinPromiseRejectFunction {
    fn internal_get_prototype_of(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<Option<Object>> {
        todo!()
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<Object>,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_is_extensible(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_prevent_extensions(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<Option<crate::ecmascript::types::PropertyDescriptor>>
    {
        todo!()
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_get(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _receiver: Value,
    ) -> crate::ecmascript::execution::JsResult<Value> {
        todo!()
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_delete(
        self,
        _agent: &mut Agent,
        _property_key: crate::ecmascript::types::PropertyKey,
    ) -> crate::ecmascript::execution::JsResult<bool> {
        todo!()
    }

    fn internal_own_property_keys(
        self,
        _agent: &mut Agent,
    ) -> crate::ecmascript::execution::JsResult<Vec<crate::ecmascript::types::PropertyKey>> {
        todo!()
    }
}

impl Index<BuiltinPromiseRejectFunction> for Agent {
    type Output = PromiseRejectFunctionHeapData;

    fn index(&self, index: BuiltinPromiseRejectFunction) -> &Self::Output {
        self.heap
            .promise_reject_functions
            .get(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_ref()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl IndexMut<BuiltinPromiseRejectFunction> for Agent {
    fn index_mut(&mut self, index: BuiltinPromiseRejectFunction) -> &mut Self::Output {
        self.heap
            .promise_reject_functions
            .get_mut(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_mut()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl CreateHeapData<PromiseRejectFunctionHeapData, BuiltinPromiseRejectFunction> for Heap {
    fn create(&mut self, data: PromiseRejectFunctionHeapData) -> BuiltinPromiseRejectFunction {
        self.promise_reject_functions.push(Some(data));
        BuiltinPromiseRejectFunction(BaseIndex::last(&self.promise_reject_functions))
    }
}

impl HeapMarkAndSweep for BuiltinPromiseRejectFunction {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promise_reject_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions
            .promise_reject_functions
            .shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for PromiseRejectFunctionHeapData {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.object_index.mark_values(queues);
        self.promise.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.promise.sweep_values(compactions);
    }
}
