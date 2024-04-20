use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyDescriptor, PropertyKey, Value,
        },
    },
    heap::indexes::EmbedderObjectIndex,
};

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmbedderObject(pub(crate) EmbedderObjectIndex);

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
        Value::EmbedderObject(val.0)
    }
}

impl From<EmbedderObject> for Object {
    fn from(val: EmbedderObject) -> Self {
        Object::EmbedderObject(val.0)
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
