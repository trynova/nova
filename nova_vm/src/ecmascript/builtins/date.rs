pub(crate) mod data;

use crate::{
    ecmascript::{
        execution::{Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyKey, Value,
        },
    },
    heap::indexes::DateIndex,
};

#[derive(Debug, Clone, Copy)]
pub struct Date(pub(crate) DateIndex);

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
    fn extensible(self, _agent: &Agent) -> bool {
        false
    }

    fn set_extensible(self, _agent: &mut Agent, _value: bool) {
        todo!()
    }

    fn prototype(self, _agent: &Agent) -> Option<Object> {
        todo!()
    }

    fn set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        todo!()
    }
}

impl InternalMethods for Date {
    fn get_prototype_of(self, _agent: &mut Agent) -> JsResult<Option<Object>> {
        todo!()
    }

    fn set_prototype_of(self, _agent: &mut Agent, _prototype: Option<Object>) -> JsResult<bool> {
        todo!()
    }

    fn is_extensible(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn prevent_extensions(self, _agent: &mut Agent) -> JsResult<bool> {
        todo!()
    }

    fn get_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
    ) -> JsResult<Option<crate::ecmascript::types::PropertyDescriptor>> {
        todo!()
    }

    fn define_own_property(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _property_descriptor: crate::ecmascript::types::PropertyDescriptor,
    ) -> JsResult<bool> {
        todo!()
    }

    fn has_property(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn get(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _receiver: Value,
    ) -> JsResult<Value> {
        todo!()
    }

    fn set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        todo!()
    }

    fn delete(self, _agent: &mut Agent, _property_key: PropertyKey) -> JsResult<bool> {
        todo!()
    }

    fn own_property_keys(self, _agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        todo!()
    }
}
