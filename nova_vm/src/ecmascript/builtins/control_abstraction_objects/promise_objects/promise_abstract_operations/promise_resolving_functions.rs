// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{
            ArgumentsList,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{Agent, JsResult},
        types::{FunctionInternalProperties, OrdinaryObject, String, Value, function_handle},
    },
    engine::context::{Bindable, GcScope, bindable_handle},
    heap::{
        ArenaAccess, ArenaAccessMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, WorkQueues, arena_vec_access, indexes::BaseIndex,
    },
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum PromiseResolvingFunctionType {
    Resolve,
    Reject,
}

/// ### [27.2.1.3.1 Promise Reject Functions](https://tc39.es/ecma262/#sec-promise-reject-functions)
///
/// A promise reject function is an anonymous built-in function that has
/// \[\[Promise\]\] and \[\[AlreadyResolved\]\] internal slots.
///
/// The "length" property of a promise reject function is 1𝔽.
#[derive(Debug, Clone)]
pub(crate) struct PromiseResolvingFunctionHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) promise_capability: PromiseCapability<'a>,
    pub(crate) resolve_type: PromiseResolvingFunctionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BuiltinPromiseResolvingFunction<'a>(
    BaseIndex<'a, PromiseResolvingFunctionHeapData<'static>>,
);
function_handle!(BuiltinPromiseResolvingFunction);
arena_vec_access!(BuiltinPromiseResolvingFunction, 'a, PromiseResolvingFunctionHeapData, promise_resolving_functions);

impl BuiltinPromiseResolvingFunction<'_> {}

impl<'a> FunctionInternalProperties<'a> for BuiltinPromiseResolvingFunction<'a> {
    fn get_name(self, _: &Agent) -> &String<'a> {
        &String::EMPTY_STRING
    }

    fn get_length(self, _: &Agent) -> u8 {
        1
    }

    #[inline(always)]
    fn get_function_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_function_backing_object(
        self,
        agent: &mut Agent,
        backing_object: OrdinaryObject<'static>,
    ) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object)
                .is_none()
        );
    }

    fn function_call<'gc>(
        self,
        agent: &mut Agent,
        _this_value: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        agent.check_call_depth(gc.nogc()).unbind()?;
        let arguments_list = arguments_list.get(0).bind(gc.nogc());
        let promise_capability = self.get(agent).promise_capability.clone();
        match self.get(agent).resolve_type {
            PromiseResolvingFunctionType::Resolve => {
                promise_capability.resolve(agent, arguments_list.unbind(), gc)
            }
            PromiseResolvingFunctionType::Reject => {
                promise_capability.reject(agent, arguments_list, gc.nogc())
            }
        };
        Ok(Value::Undefined)
    }
}

impl<'a> CreateHeapData<PromiseResolvingFunctionHeapData<'a>, BuiltinPromiseResolvingFunction<'a>>
    for Heap
{
    fn create(
        &mut self,
        data: PromiseResolvingFunctionHeapData<'a>,
    ) -> BuiltinPromiseResolvingFunction<'a> {
        self.promise_resolving_functions.push(data.unbind());
        self.alloc_counter +=
            core::mem::size_of::<Option<PromiseResolvingFunctionHeapData<'static>>>();

        BuiltinPromiseResolvingFunction(BaseIndex::last(&self.promise_resolving_functions))
    }
}

impl HeapMarkAndSweep for BuiltinPromiseResolvingFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.promise_resolving_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .promise_resolving_functions
            .shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for BuiltinPromiseResolvingFunction<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .promise_resolving_functions
            .shift_weak_index(self.0)
            .map(Self)
    }
}

bindable_handle!(PromiseResolvingFunctionHeapData);

impl HeapMarkAndSweep for PromiseResolvingFunctionHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            promise_capability,
            resolve_type: _,
        } = self;
        object_index.mark_values(queues);
        promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            promise_capability,
            resolve_type: _,
        } = self;
        object_index.sweep_values(compactions);
        promise_capability.sweep_values(compactions);
    }
}
