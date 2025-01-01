// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::TryResult;
use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{
        indexes::{BaseIndex, ProxyIndex},
        CreateHeapData, Heap, HeapMarkAndSweep,
    },
};

use self::data::ProxyHeapData;

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
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Option<Object>> {
        Ok(self.internal_prototype(agent))
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
