// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{ControlFlow, Index, IndexMut};

use abstract_operations::validate_non_revoked_proxy;

use crate::ecmascript::abstract_operations::operations_on_objects::{call, get_method};
use crate::ecmascript::abstract_operations::testing_and_comparison::{
    same_value, try_is_extensible,
};
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::TryResult;
use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyDescriptor, PropertyKey, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::{BaseIndex, ProxyIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::ProxyHeapData;

pub(crate) mod abstract_operations;
pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Proxy(pub(crate) ProxyIndex);

impl Proxy {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<Proxy> for ProxyIndex {
    fn from(val: Proxy) -> Self {
        val.0
    }
}

impl From<ProxyIndex> for Proxy {
    fn from(value: ProxyIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for Proxy {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Proxy {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Proxy> for Value {
    fn from(val: Proxy) -> Self {
        Value::Proxy(val)
    }
}

impl From<Proxy> for Object {
    fn from(val: Proxy) -> Self {
        Object::Proxy(val)
    }
}

impl InternalSlots for Proxy {
    #[inline(always)]
    fn get_backing_object(self, _agent: &Agent) -> Option<OrdinaryObject<'static>> {
        todo!()
    }

    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        todo!()
    }

    fn create_backing_object(self, _agent: &mut Agent) -> OrdinaryObject<'static> {
        todo!()
    }

    fn internal_extensible(self, _agent: &Agent) -> bool {
        todo!();
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!();
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        todo!();
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!();
    }
}

impl InternalMethods for Proxy {
    fn try_get_prototype_of(
        self,
        agent: &mut Agent,
        _gc: NoGcScope<'_, '_>,
    ) -> TryResult<Option<Object>> {
        TryResult::Continue(self.internal_prototype(agent))
    }

    fn internal_get_prototype_of(
        self,
        agent: &mut Agent,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Option<Object>> {
        // 1. Perform ? ValidateNonRevokedProxy(O).
        validate_non_revoked_proxy(agent, self, gc.nogc())?;

        let proxy_data = &agent[self];

        // 2. Let target be O.[[ProxyTarget]].
        let target = proxy_data.target;

        // 3. Let handler be O.[[ProxyHandler]].
        let handler = proxy_data.handler;

        // 4. Assert: handler is an Object.
        assert!(Object::try_from(handler.unwrap()).is_ok());

        // 5. Let trap be ? GetMethod(handler, "getPrototypeOf").
        let trap = get_method(
            agent,
            handler.unwrap(),
            BUILTIN_STRING_MEMORY.getPrototypeOf.into(),
            gc.reborrow(),
        )?;

        // 6. If trap is undefined, then
        if trap.is_none() {
            // a. Return ? target.[[GetPrototypeOf]]().
            return Ok(Object::internal_prototype(
                Object::try_from(target.unwrap()).unwrap(),
                agent,
            ));
        }

        // 7. Let handlerProto be ? Call(trap, handler, « target »).
        let handler_proto = call(
            agent,
            trap.into(),
            handler.into(),
            Some(ArgumentsList(&[target.into()])),
            gc.reborrow(),
        )?;

        // 8. If handlerProto is not an Object and handlerProto is not null, throw a TypeError exception.
        if !handler_proto.is_object() && !handler_proto.is_null() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Handler prototype must be an object or null",
                gc.nogc(),
            ));
        }

        // 9. Let extensibleTarget be ? IsExtensible(target).
        let extensible_target = if let ControlFlow::Continue(result) =
            try_is_extensible(agent, Object::try_from(target.unwrap()).unwrap(), gc.nogc())
        {
            result
        } else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Unexpected break in ControlFlow",
                gc.nogc(),
            ));
        };

        // 10. If extensibleTarget is true, return handlerProto.
        if extensible_target {
            return Ok(Object::internal_prototype(
                Object::try_from(handler_proto).unwrap(),
                agent,
            ));
        }

        // 11. Let targetProto be ? target.[[GetPrototypeOf]]().
        let target_proto = Proxy::internal_get_prototype_of(self, agent, gc.reborrow())?;

        // 12. If SameValue(handlerProto, targetProto) is false, throw a TypeError exception.
        if !same_value(agent, handler_proto, target_proto) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Unexpected break in ControlFlow",
                gc.nogc(),
            ));
        }

        // 13. Return handlerProto.
        Ok(Object::internal_prototype(
            Object::try_from(handler_proto).unwrap(),
            agent,
        ))
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<Object>,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_is_extensible(self, agent: &mut Agent, _gc: GcScope<'_, '_>) -> JsResult<bool> {
        Ok(self.internal_extensible(agent))
    }

    fn internal_prevent_extensions(
        self,
        agent: &mut Agent,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        self.internal_set_extensible(agent, false);
        Ok(true)
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Option<PropertyDescriptor>> {
        todo!();
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: PropertyDescriptor,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_get(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _receiver: Value,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!();
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_delete(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_own_property_keys<'a>(
        self,
        _agent: &mut Agent,
        _gc: GcScope<'a, '_>,
    ) -> JsResult<Vec<PropertyKey<'a>>> {
        todo!();
    }

    fn internal_call(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: super::ArgumentsList,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn internal_construct(
        self,
        _agent: &mut Agent,
        _arguments_list: super::ArgumentsList,
        _new_target: crate::ecmascript::types::Function,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Object> {
        todo!()
    }
}

impl Index<Proxy> for Agent {
    type Output = ProxyHeapData;

    fn index(&self, index: Proxy) -> &Self::Output {
        &self.heap.proxys[index]
    }
}

impl IndexMut<Proxy> for Agent {
    fn index_mut(&mut self, index: Proxy) -> &mut Self::Output {
        &mut self.heap.proxys[index]
    }
}

impl Index<Proxy> for Vec<Option<ProxyHeapData>> {
    type Output = ProxyHeapData;

    fn index(&self, index: Proxy) -> &Self::Output {
        self.get(index.get_index())
            .expect("Proxy out of bounds")
            .as_ref()
            .expect("Proxy slot empty")
    }
}

impl IndexMut<Proxy> for Vec<Option<ProxyHeapData>> {
    fn index_mut(&mut self, index: Proxy) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Proxy out of bounds")
            .as_mut()
            .expect("Proxy slot empty")
    }
}

impl CreateHeapData<ProxyHeapData, Proxy> for Heap {
    fn create(&mut self, data: ProxyHeapData) -> Proxy {
        self.proxys.push(Some(data));
        Proxy(ProxyIndex::last(&self.proxys))
    }
}

impl HeapMarkAndSweep for Proxy {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        queues.proxys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.proxys.shift_index(&mut self.0);
    }
}
