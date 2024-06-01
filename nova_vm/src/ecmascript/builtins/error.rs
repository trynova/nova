mod data;

use std::ops::{Index, IndexMut};

pub(crate) use data::ErrorHeapData;

use crate::{
    ecmascript::{
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyKey, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::ErrorIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Error(pub(crate) ErrorIndex);

impl Error {
    pub(crate) const fn _def() -> Self {
        Self(ErrorIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl IntoValue for Error {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl From<Error> for Value {
    fn from(value: Error) -> Self {
        Value::Error(value)
    }
}

impl IntoObject for Error {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Error> for Object {
    fn from(value: Error) -> Self {
        Object::Error(value)
    }
}

impl TryFrom<Value> for Error {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, ()> {
        match value {
            Value::Error(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl TryFrom<Object> for Error {
    type Error = ();

    fn try_from(value: Object) -> Result<Self, ()> {
        match value {
            Object::Error(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl OrdinaryObjectInternalSlots for Error {
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

impl InternalMethods for Error {
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
        if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.toString) {
            agent
                .current_realm()
                .intrinsics()
                .error_prototype()
                .internal_get(agent, property_key, receiver)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
            match agent[self].kind {
                ExceptionType::AggregateError => {
                    Ok(BUILTIN_STRING_MEMORY.AggregateError.into_value())
                }
                ExceptionType::Error => Ok(BUILTIN_STRING_MEMORY.Error.into_value()),
                ExceptionType::EvalError => Ok(BUILTIN_STRING_MEMORY.EvalError.into_value()),
                ExceptionType::RangeError => Ok(BUILTIN_STRING_MEMORY.RangeError.into_value()),
                ExceptionType::ReferenceError => {
                    Ok(BUILTIN_STRING_MEMORY.ReferenceError.into_value())
                }
                ExceptionType::SyntaxError => Ok(BUILTIN_STRING_MEMORY.SyntaxError.into_value()),
                ExceptionType::TypeError => Ok(BUILTIN_STRING_MEMORY.TypeError.into_value()),
                ExceptionType::UriError => Ok(BUILTIN_STRING_MEMORY.URIError.into_value()),
            }
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
            Ok(agent[self]
                .message
                .map_or(Value::Undefined, |message| message.into_value()))
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
            Ok(agent[self].cause.unwrap_or(Value::Undefined))
        } else {
            Ok(Value::Undefined)
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

impl HeapMarkAndSweep for Error {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.errors.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 =
            ErrorIndex::from_u32(self_index - compactions.errors.get_shift_for_index(self_index));
    }
}

impl CreateHeapData<ErrorHeapData, Error> for Heap {
    fn create(&mut self, data: ErrorHeapData) -> Error {
        self.errors.push(Some(data));
        Error(ErrorIndex::last(&self.errors))
    }
}

impl Index<Error> for Agent {
    type Output = ErrorHeapData;

    fn index(&self, index: Error) -> &Self::Output {
        self.heap
            .errors
            .get(index.get_index())
            .expect("Error out of bounds")
            .as_ref()
            .expect("Error slot empty")
    }
}

impl IndexMut<Error> for Agent {
    fn index_mut(&mut self, index: Error) -> &mut Self::Output {
        self.heap
            .errors
            .get_mut(index.get_index())
            .expect("Error out of bounds")
            .as_mut()
            .expect("Error slot empty")
    }
}
