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
        indexes::{BaseIndex, EmbedderObjectIndex},
        HeapMarkAndSweep,
    },
};

use self::data::EmbedderObjectHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EmbedderObject<'gen>(pub(crate) EmbedderObjectIndex<'gen>);

impl<'gen> EmbedderObject<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl<'gen> From<EmbedderObject<'gen>> for EmbedderObjectIndex<'gen> {
    fn from(val: EmbedderObject<'gen>) -> Self {
        val.0
    }
}

impl<'gen> From<EmbedderObjectIndex<'gen>> for EmbedderObject<'gen> {
    fn from(value: EmbedderObjectIndex<'gen>) -> Self {
        Self(value)
    }
}

impl<'gen> IntoValue<'gen> for EmbedderObject<'gen> {
    fn into_value(self) -> Value<'gen> {
        self.into()
    }
}

impl<'gen> IntoObject<'gen> for EmbedderObject<'gen> {
    fn into_object(self) -> Object<'gen> {
        self.into()
    }
}

impl<'gen> From<EmbedderObject<'gen>> for Value<'gen> {
    fn from(val: EmbedderObject<'gen>) -> Self {
        Value::EmbedderObject(val)
    }
}

impl<'gen> From<EmbedderObject<'gen>> for Object<'gen> {
    fn from(val: EmbedderObject<'gen>) -> Self {
        Object::EmbedderObject(val)
    }
}

impl<'gen> InternalSlots<'gen> for EmbedderObject<'gen> {
    #[inline(always)]
    fn get_backing_object(
        self,
        _agent: &Agent<'gen>,
    ) -> Option<crate::ecmascript::types::OrdinaryObject<'gen>> {
        todo!();
    }

    fn create_backing_object<'b>(self, _agent: &mut Agent<'gen>) -> crate::ecmascript::types::OrdinaryObject<'gen> {
        todo!();
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

impl<'gen> InternalMethods<'gen> for EmbedderObject<'gen> {
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
}

impl<'gen> Index<EmbedderObject<'gen>> for Agent<'gen> {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObject<'gen>) -> &Self::Output {
        &self.heap.embedder_objects[index]
    }
}

impl<'gen> IndexMut<EmbedderObject<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: EmbedderObject<'gen>) -> &mut Self::Output {
        &mut self.heap.embedder_objects[index]
    }
}

impl<'gen> Index<EmbedderObject<'gen>> for Vec<Option<EmbedderObjectHeapData>> {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObject<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("EmbedderObject out of bounds")
            .as_ref()
            .expect("EmbedderObject slot empty")
    }
}

impl<'gen> IndexMut<EmbedderObject<'gen>> for Vec<Option<EmbedderObjectHeapData>> {
    fn index_mut(&mut self, index: EmbedderObject<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("EmbedderObject out of bounds")
            .as_mut()
            .expect("EmbedderObject slot empty")
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for EmbedderObject<'gen> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues<'gen>) {
        queues.embedder_objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        compactions.embedder_objects.shift_index(&mut self.0);
    }
}
