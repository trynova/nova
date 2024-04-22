use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::{indexes::ProxyIndex, Heap},
};

use self::data::ProxyHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proxy(pub(crate) ProxyIndex);

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
        Value::Proxy(val.0)
    }
}

impl From<Proxy> for Object {
    fn from(val: Proxy) -> Self {
        Object::Proxy(val.0)
    }
}

impl Index<Proxy> for Agent {
    type Output = ProxyHeapData;

    fn index(&self, index: Proxy) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<Proxy> for Agent {
    fn index_mut(&mut self, index: Proxy) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<Proxy> for Heap {
    type Output = ProxyHeapData;

    fn index(&self, index: Proxy) -> &Self::Output {
        self.proxys
            .get(index.0.into_index())
            .expect("Proxy out of bounds")
            .as_ref()
            .expect("Proxy slot empty")
    }
}

impl IndexMut<Proxy> for Heap {
    fn index_mut(&mut self, index: Proxy) -> &mut Self::Output {
        self.proxys
            .get_mut(index.0.into_index())
            .expect("Proxy out of bounds")
            .as_mut()
            .expect("Proxy slot empty")
    }
}

impl OrdinaryObjectInternalSlots for Proxy {
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
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(self.internal_prototype(agent))
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<Object>,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        Ok(self.internal_extensible(agent))
    }

    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        self.internal_set_extensible(agent, false);
        Ok(true)
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        todo!();
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_get(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _receiver: Value,
    ) -> JsResult<Value> {
        todo!();
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!();
    }

    fn internal_delete(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!();
    }

    fn internal_own_property_keys(self, _agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        todo!();
    }
}
