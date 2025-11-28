// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        builtins::{
            ArgumentsList,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{Agent, JsResult},
        types::{Function, FunctionInternalProperties, Object, OrdinaryObject, String, Value},
    },
    engine::{
        context::{Bindable, GcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        WorkQueues, indexes::BaseIndex,
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
pub struct PromiseResolvingFunctionHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) promise_capability: PromiseCapability<'a>,
    pub(crate) resolve_type: PromiseResolvingFunctionType,
}

pub(crate) type BuiltinPromiseResolvingFunctionIndex<'a> =
    BaseIndex<'a, PromiseResolvingFunctionHeapData<'static>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BuiltinPromiseResolvingFunction<'a>(pub(crate) BuiltinPromiseResolvingFunctionIndex<'a>);

impl BuiltinPromiseResolvingFunction<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

bindable_handle!(BuiltinPromiseResolvingFunction);

impl<'a> From<BuiltinPromiseResolvingFunction<'a>> for Function<'a> {
    fn from(value: BuiltinPromiseResolvingFunction<'a>) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl<'a> From<BuiltinPromiseResolvingFunction<'a>> for Object<'a> {
    fn from(value: BuiltinPromiseResolvingFunction) -> Self {
        Self::BuiltinPromiseResolvingFunction(value.unbind())
    }
}

impl<'a> From<BuiltinPromiseResolvingFunction<'a>> for Value<'a> {
    fn from(value: BuiltinPromiseResolvingFunction<'a>) -> Self {
        Self::BuiltinPromiseResolvingFunction(value)
    }
}

impl<'a> FunctionInternalProperties<'a> for BuiltinPromiseResolvingFunction<'a> {
    fn get_name(self, _: &Agent) -> &String<'a> {
        &String::EMPTY_STRING
    }

    fn get_length(self, _: &Agent) -> u8 {
        1
    }

    #[inline(always)]
    fn get_function_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_function_backing_object(
        self,
        agent: &mut Agent,
        backing_object: OrdinaryObject<'static>,
    ) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
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
        let promise_capability = agent[self].promise_capability.clone();
        match agent[self].resolve_type {
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

impl Index<BuiltinPromiseResolvingFunction<'_>> for Agent {
    type Output = PromiseResolvingFunctionHeapData<'static>;

    fn index(&self, index: BuiltinPromiseResolvingFunction) -> &Self::Output {
        &self.heap.promise_resolving_functions[index]
    }
}

impl IndexMut<BuiltinPromiseResolvingFunction<'_>> for Agent {
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction) -> &mut Self::Output {
        &mut self.heap.promise_resolving_functions[index]
    }
}

impl Index<BuiltinPromiseResolvingFunction<'_>> for Vec<PromiseResolvingFunctionHeapData<'static>> {
    type Output = PromiseResolvingFunctionHeapData<'static>;

    fn index(&self, index: BuiltinPromiseResolvingFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
    }
}

impl IndexMut<BuiltinPromiseResolvingFunction<'_>>
    for Vec<PromiseResolvingFunctionHeapData<'static>>
{
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
    }
}

impl Rootable for BuiltinPromiseResolvingFunction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::BuiltinPromiseResolvingFunction(
            value.unbind(),
        ))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BuiltinPromiseResolvingFunction(d) => Some(d),
            _ => None,
        }
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
