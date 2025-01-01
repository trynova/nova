// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::ecmascript::types::{function_try_get, function_try_has_property, function_try_set};
use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::TryResult;
use crate::{
    ecmascript::{
        builtins::{control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability, ArgumentsList},
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            function_create_backing_object, function_internal_define_own_property, function_internal_delete, function_internal_get, function_internal_get_own_property, function_internal_has_property, function_internal_own_property_keys, function_internal_set, Function, FunctionInternalProperties, InternalMethods, InternalSlots, IntoFunction, IntoObject, IntoValue, Object, OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value
        },
    },
    heap::{
        indexes::BaseIndex, CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum PromiseResolvingFunctionType {
    Resolve,
    Reject,
}

/// ### [27.2.1.3.1 Promise Reject Functions]()
///
/// A promise reject function is an anonymous built-in function that has
/// \[\[Promise\]\] and \[\[AlreadyResolved\]\] internal slots.
///
/// The "length" property of a promise reject function is 1𝔽.
#[derive(Debug, Clone, Copy)]
pub struct PromiseResolvingFunctionHeapData {
    pub(crate) object_index: Option<OrdinaryObject<'static>>,
    pub(crate) promise_capability: PromiseCapability,
    pub(crate) resolve_type: PromiseResolvingFunctionType,
}

pub(crate) type BuiltinPromiseResolvingFunctionIndex =
    BaseIndex<'static, PromiseResolvingFunctionHeapData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinPromiseResolvingFunction(pub(crate) BuiltinPromiseResolvingFunctionIndex);

impl BuiltinPromiseResolvingFunction {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<BuiltinPromiseResolvingFunction> for Function {
    fn from(value: BuiltinPromiseResolvingFunction) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl IntoFunction for BuiltinPromiseResolvingFunction {
    fn into_function(self) -> Function {
        self.into()
    }
}

impl From<BuiltinPromiseResolvingFunction> for Object {
    fn from(value: BuiltinPromiseResolvingFunction) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl IntoObject for BuiltinPromiseResolvingFunction {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<BuiltinPromiseResolvingFunction> for Value {
    fn from(value: BuiltinPromiseResolvingFunction) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl IntoValue for BuiltinPromiseResolvingFunction {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl FunctionInternalProperties for BuiltinPromiseResolvingFunction {
    fn get_name(self, _: &Agent) -> String<'static> {
        String::EMPTY_STRING
    }

    fn get_length(self, _: &Agent) -> u8 {
        1
    }
}

impl InternalSlots for BuiltinPromiseResolvingFunction {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        function_create_backing_object(self, agent)
    }
}

impl InternalMethods for BuiltinPromiseResolvingFunction {
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _gc: NoGcScope<'_, '_>,
    ) -> TryResult<Option<PropertyDescriptor>> {
        TryResult::Continue(function_internal_get_own_property(
            self,
            agent,
            property_key,
        ))
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        TryResult::Continue(function_internal_define_own_property(
            self,
            agent,
            property_key,
            property_descriptor,
            gc,
        ))
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        function_try_has_property(self, agent, property_key, gc)
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        function_internal_has_property(self, agent, property_key, gc)
    }

    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<Value> {
        function_try_get(self, agent, property_key, receiver, gc)
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        function_internal_get(self, agent, property_key, receiver, gc)
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        function_try_set(self, agent, property_key, value, receiver, gc)
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        function_internal_set(self, agent, property_key, value, receiver, gc)
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        TryResult::Continue(function_internal_delete(self, agent, property_key, gc))
    }

    fn try_own_property_keys<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<Vec<PropertyKey<'a>>> {
        TryResult::Continue(function_internal_own_property_keys(self, agent, gc))
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        _this_value: Value,
        args: ArgumentsList,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        let arg = args.get(0);
        let promise_capability = agent[self].promise_capability;
        match agent[self].resolve_type {
            PromiseResolvingFunctionType::Resolve => promise_capability.resolve(agent, arg, gc),
            PromiseResolvingFunctionType::Reject => promise_capability.reject(agent, arg),
        };
        Ok(Value::Undefined)
    }
}

impl Index<BuiltinPromiseResolvingFunction> for Agent {
    type Output = PromiseResolvingFunctionHeapData;

    fn index(&self, index: BuiltinPromiseResolvingFunction) -> &Self::Output {
        &self.heap.promise_resolving_functions[index]
    }
}

impl IndexMut<BuiltinPromiseResolvingFunction> for Agent {
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction) -> &mut Self::Output {
        &mut self.heap.promise_resolving_functions[index]
    }
}

impl Index<BuiltinPromiseResolvingFunction> for Vec<Option<PromiseResolvingFunctionHeapData>> {
    type Output = PromiseResolvingFunctionHeapData;

    fn index(&self, index: BuiltinPromiseResolvingFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_ref()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl IndexMut<BuiltinPromiseResolvingFunction> for Vec<Option<PromiseResolvingFunctionHeapData>> {
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_mut()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl CreateHeapData<PromiseResolvingFunctionHeapData, BuiltinPromiseResolvingFunction> for Heap {
    fn create(
        &mut self,
        data: PromiseResolvingFunctionHeapData,
    ) -> BuiltinPromiseResolvingFunction {
        self.promise_resolving_functions.push(Some(data));
        BuiltinPromiseResolvingFunction(BaseIndex::last(&self.promise_resolving_functions))
    }
}

impl HeapMarkAndSweep for BuiltinPromiseResolvingFunction {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.promise_resolving_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions
            .promise_resolving_functions
            .shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for PromiseResolvingFunctionHeapData {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.object_index.mark_values(queues);
        self.promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.promise_capability.sweep_values(compactions);
    }
}
