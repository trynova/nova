use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::indexes::{BaseIndex, EmbedderObjectIndex},
};

use self::data::EmbedderObjectHeapData;

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EmbedderObject(pub(crate) EmbedderObjectIndex);

impl EmbedderObject {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl From<EmbedderObject> for EmbedderObjectIndex {
    fn from(val: EmbedderObject) -> Self {
        val.0
    }
}

impl From<EmbedderObjectIndex> for EmbedderObject {
    fn from(value: EmbedderObjectIndex) -> Self {
        Self(value)
    }
}

impl IntoValue for EmbedderObject {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for EmbedderObject {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<EmbedderObject> for Value {
    fn from(val: EmbedderObject) -> Self {
        Value::EmbedderObject(val)
    }
}

impl From<EmbedderObject> for Object {
    fn from(val: EmbedderObject) -> Self {
        Object::EmbedderObject(val)
    }
}

impl OrdinaryObjectInternalSlots for EmbedderObject {
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

impl InternalMethods for EmbedderObject {
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

impl Index<EmbedderObjectIndex> for Agent {
    type Output = EmbedderObjectHeapData;

    fn index(&self, index: EmbedderObjectIndex) -> &Self::Output {
        self.heap
            .embedder_objects
            .get(index.into_index())
            .expect("EmbedderObjectIndex out of bounds")
            .as_ref()
            .expect("EmbedderObjectIndex slot empty")
    }
}

impl IndexMut<EmbedderObjectIndex> for Agent {
    fn index_mut(&mut self, index: EmbedderObjectIndex) -> &mut Self::Output {
        self.heap
            .embedder_objects
            .get_mut(index.into_index())
            .expect("EmbedderObjectIndex out of bounds")
            .as_mut()
            .expect("EmbedderObjectIndex slot empty")
    }
}
