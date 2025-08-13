// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};
use std::ops::ControlFlow;

use crate::{
    ecmascript::{
        builtins::{
            ArgumentsList, ordinary::caches::PropertyLookupCache,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            Function, FunctionInternalProperties, InternalMethods, InternalSlots, NoCache, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, SetCachedProps, SetCachedResult,
            String, TryGetContinue, TryGetResult, Value, function_create_backing_object,
            function_get_cached, function_internal_define_own_property, function_internal_delete,
            function_internal_get, function_internal_get_own_property,
            function_internal_has_property, function_internal_own_property_keys,
            function_internal_set, function_set_cached, function_try_get,
            function_try_has_property, function_try_set,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
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

/// ### [27.2.1.3.1 Promise Reject Functions]()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinPromiseResolvingFunction<'a>(pub(crate) BuiltinPromiseResolvingFunctionIndex<'a>);

impl BuiltinPromiseResolvingFunction<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for BuiltinPromiseResolvingFunction<'_> {
    type Of<'a> = BuiltinPromiseResolvingFunction<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

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
    fn get_name(self, _: &Agent) -> String<'static> {
        String::EMPTY_STRING
    }

    fn get_length(self, _: &Agent) -> u8 {
        1
    }
}

impl<'a> InternalSlots<'a> for BuiltinPromiseResolvingFunction<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        function_create_backing_object(self, agent)
    }
}

impl<'a> InternalMethods<'a> for BuiltinPromiseResolvingFunction<'a> {
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        TryResult::Continue(function_internal_get_own_property(
            self,
            agent,
            property_key,
            gc,
        ))
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match function_internal_define_own_property(
            self,
            agent,
            property_key,
            property_descriptor,
            gc,
        ) {
            Ok(b) => TryResult::Continue(b),
            Err(_) => TryResult::Break(()),
        }
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        function_try_has_property(self, agent, property_key, gc)
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        function_internal_has_property(self, agent, property_key, gc)
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        function_try_get(self, agent, property_key, receiver, cache, gc)
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        function_internal_get(self, agent, property_key, receiver, gc)
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        function_try_set(self, agent, property_key, value, receiver, gc)
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        function_internal_set(self, agent, property_key, value, receiver, gc)
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        TryResult::Continue(function_internal_delete(self, agent, property_key, gc))
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        TryResult::Continue(function_internal_own_property_keys(self, agent, gc))
    }

    fn get_cached<'gc>(
        self,
        agent: &mut Agent,
        p: PropertyKey,
        cache: PropertyLookupCache,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<TryGetContinue<'gc>, NoCache> {
        function_get_cached(self, agent, p, cache, gc)
    }

    fn set_cached<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
        function_set_cached(self, agent, props, gc)
    }

    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        _this_value: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
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

impl Index<BuiltinPromiseResolvingFunction<'_>>
    for Vec<Option<PromiseResolvingFunctionHeapData<'static>>>
{
    type Output = PromiseResolvingFunctionHeapData<'static>;

    fn index(&self, index: BuiltinPromiseResolvingFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_ref()
            .expect("BuiltinPromiseRejectFunction slot empty")
    }
}

impl IndexMut<BuiltinPromiseResolvingFunction<'_>>
    for Vec<Option<PromiseResolvingFunctionHeapData<'static>>>
{
    fn index_mut(&mut self, index: BuiltinPromiseResolvingFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinPromiseRejectFunction out of bounds")
            .as_mut()
            .expect("BuiltinPromiseRejectFunction slot empty")
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
        self.promise_resolving_functions.push(Some(data.unbind()));
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

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for PromiseResolvingFunctionHeapData<'_> {
    type Of<'a> = PromiseResolvingFunctionHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for PromiseResolvingFunctionHeapData<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.object_index.mark_values(queues);
        self.promise_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.object_index.sweep_values(compactions);
        self.promise_capability.sweep_values(compactions);
    }
}
