pub(crate) mod data;

use std::ops::Deref;

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject,
            OrdinaryObjectInternalSlots, PropertyKey, Value,
        },
    },
    heap::{indexes::DateIndex, GetHeapData},
};

#[derive(Debug, Clone, Copy)]
pub struct Date(pub(crate) DateIndex);

impl Deref for Date {
    type Target = DateIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<DateIndex> for Date {
    fn from(value: DateIndex) -> Self {
        Self(value)
    }
}

impl From<DateIndex> for Value {
    fn from(value: DateIndex) -> Self {
        Self::Date(value)
    }
}

impl IntoValue for Date {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl From<Date> for Value {
    fn from(value: Date) -> Self {
        Value::Date(value.0)
    }
}

impl IntoObject for Date {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Date> for Object {
    fn from(value: Date) -> Self {
        Object::Date(value.0)
    }
}

impl TryFrom<Value> for Date {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, ()> {
        match value {
            Value::Date(idx) => Ok(idx.into()),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for Date {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, ()> {
        match value {
            Object::Date(idx) => Ok(idx.into()),
            _ => Err(()),
        }
    }
}

impl OrdinaryObjectInternalSlots for Date {
    fn internal_extensible(self, _agent: &Agent) -> bool {
        false
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!()
    }

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        todo!()
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!()
    }
}

impl InternalMethods for Date {
    fn internal_get_prototype_of(self, _agent: &mut Agent) -> JsResult<Option<Object>> {
        todo!()
    }

    fn internal_set_prototype_of(
        self,
        _agent: &mut Agent,
        _prototype: Option<Object>,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_is_extensible(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn internal_prevent_extensions(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn internal_get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<Option<crate::ecmascript::types::PropertyDescriptor>> {
        todo!()
    }

    fn internal_define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_has_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        if let Some(object_index) = agent.heap.get(*self).object_index {
            OrdinaryObject::from(object_index).internal_get(agent, property_key, receiver)
        } else {
            agent
                .current_realm()
                .intrinsics()
                .date_prototype()
                .internal_get(agent, property_key, receiver)
        }
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!()
    }

    fn internal_delete(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn internal_own_property_keys(self, _agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        todo!()
    }
}
