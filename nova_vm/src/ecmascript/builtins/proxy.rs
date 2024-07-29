// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, PropertyDescriptor,
            PropertyKey, Value,
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
pub struct Proxy<'gen>(pub(crate) ProxyIndex<'gen>);

impl<'gen> Proxy<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<Proxy<'gen>> for ProxyIndex<'gen> {
    fn from(val: Proxy<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<ProxyIndex<'gen>> for Proxy<'gen> {
    fn from(value: ProxyIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for Proxy<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for Proxy<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<Proxy<'gen>> for Value<'gen> {
    fn from(val: Proxy<'gen>) -> Self {
        Value::Proxy(val)
    }
}

impl<'gen> From<Proxy<'gen>> for Object<'gen> {
    fn from(val: Proxy<'gen>) -> Self {
        Object::Proxy(val)
    }
}

impl<'gen> InternalSlots<'gen> for Proxy<'gen> {
    #[inline(always)]
    fn get_backing_object(
        self,
        _agent: &Agent<'gen>,
    ) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        todo!()
    }

    fn create_backing_object(self, _agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
        todo!()
    }

    fn internal_extensible(self, _agent: &Agent<'gen>) -> bool {
        todo!();
    }

    fn internal_set_extensible(self, _agent: &mut Agent<'gen>, _value: bool) {
        todo!();
    }

    fn internal_prototype(self, _agent: &Agent<'gen>) -> Option<Object<'gen>> {
        todo!();
    }

    fn internal_set_prototype(self, _agent: &mut Agent<'gen>, _prototype: Option<Object<'gen>>) {
        todo!();
    }
}

impl<'gen> InternalMethods<'gen> for Proxy<'gen> {
    fn internal_get_prototype_of(self, agent: &mut Agent<'gen>) -> JsResult<'gen, Option<Object<'gen>>> {
        Ok(self.internal_prototype(agent))
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent<'gen>,
        _prototype: Option<Object<'gen>>,
    ) -> JsResult<'gen, bool> {
        todo!();
    }

    fn internal_is_extensible(self, agent: &mut Agent<'gen>) -> JsResult<'gen, bool> {
        Ok(self.internal_extensible(agent))
    }

    fn internal_prevent_extensions(self, agent: &mut Agent<'gen>) -> JsResult<'gen, bool> {
        self.internal_set_extensible(agent, false);
        Ok(true)
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent<'gen>,
        _property_key: PropertyKey<'gen>,
    ) -> JsResult<'gen, Option<PropertyDescriptor<'gen>>> {
        todo!();
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent<'gen>,
        _property_key: PropertyKey<'gen>,
        _property_descriptor: PropertyDescriptor<'gen>,
    ) -> JsResult<'gen, bool> {
        todo!();
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent<'gen>,
        _property_key: PropertyKey<'gen>,
    ) -> JsResult<'gen, bool> {
        todo!();
    }

    fn internal_get(
        self,
        _agent: &mut Agent<'gen>,
        _property_key: PropertyKey<'gen>,
        _receiver: Value<'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn internal_set(
        self,
        _agent: &mut Agent<'gen>,
        _property_key: PropertyKey<'gen>,
        _value: Value<'gen>,
        _receiver: Value<'gen>,
    ) -> JsResult<'gen, bool> {
        todo!();
    }

    fn internal_delete(self, _agent: &mut Agent<'gen>, _property_key: PropertyKey<'gen>) -> JsResult<'gen, bool> {
        todo!();
    }

    fn internal_own_property_keys(self, _agent: &mut Agent<'gen>) -> JsResult<'gen, Vec<PropertyKey<'gen>>> {
        todo!();
    }

    fn internal_call(
        self,
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments_list: super::ArgumentsList,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn internal_construct(
        self,
        _agent: &mut Agent<'gen>,
        _arguments_list: super::ArgumentsList,
        _new_target: crate::ecmascript::types::Function,
    ) -> JsResult<'gen, Object<'gen>> {
        todo!()
    }
}

impl<'gen> Index<Proxy<'gen>> for Agent<'gen> {
    type Output = ProxyHeapData<'gen>;

    fn index(&self, index: Proxy<'gen>) -> &Self::Output {
        &self.heap.proxys[index]
    }
}

impl<'gen> IndexMut<Proxy<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: Proxy<'gen>) -> &mut Self::Output {
        &mut self.heap.proxys[index]
    }
}

impl<'gen> Index<Proxy<'gen>> for Vec<Option<ProxyHeapData<'gen>>> {
    type Output = ProxyHeapData<'gen>;

    fn index(&self, index: Proxy<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Proxy out of bounds")
            .as_ref()
            .expect("Proxy slot empty")
    }
}

impl<'gen> IndexMut<Proxy<'gen>> for Vec<Option<ProxyHeapData<'gen>>> {
    fn index_mut(&mut self, index: Proxy<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Proxy out of bounds")
            .as_mut()
            .expect("Proxy slot empty")
    }
}

impl<'gen> CreateHeapData<ProxyHeapData<'gen>, Proxy<'gen>> for Heap<'gen> {
    fn create(&mut self, data: ProxyHeapData<'gen>) -> Proxy<'gen> {
        self.proxys.push(Some(data));
        Proxy(ProxyIndex::last(&self.proxys))
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for Proxy<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.proxys.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.proxys.shift_index(&mut self.0);
    }
}
