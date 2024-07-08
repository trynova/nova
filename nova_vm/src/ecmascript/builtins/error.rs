// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use std::ops::{Index, IndexMut};

pub(crate) use data::ErrorHeapData;

use crate::{
    ecmascript::{
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            PropertyDescriptor, PropertyKey, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{
        indexes::ErrorIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, ObjectEntry,
        ObjectEntryPropertyDescriptor, WorkQueues,
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

impl InternalSlots for Error {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Error;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<crate::ecmascript::types::OrdinaryObject> {
        agent[self].object_index
    }

    fn create_backing_object(self, agent: &mut Agent) -> crate::ecmascript::types::OrdinaryObject {
        let prototype = self.internal_prototype(agent).unwrap();
        let message_entry = agent[self].message.map(|message| ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            value: ObjectEntryPropertyDescriptor::Data {
                value: message.into_value(),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        });
        let cause_entry = agent[self].cause.map(|cause| ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
            value: ObjectEntryPropertyDescriptor::Data {
                value: cause,
                writable: false,
                enumerable: false,
                configurable: true,
            },
        });
        let (keys, values) =
            if let (Some(message_entry), Some(cause_entry)) = (message_entry, cause_entry) {
                agent
                    .heap
                    .elements
                    .create_object_entries(&[message_entry, cause_entry])
            } else if let Some(message_entry) = message_entry {
                agent.heap.elements.create_object_entries(&[message_entry])
            } else if let Some(cause_entry) = cause_entry {
                agent.heap.elements.create_object_entries(&[cause_entry])
            } else {
                agent.heap.elements.create_object_entries(&[])
            };
        let backing_object = agent.heap.create(ObjectHeapData {
            extensible: true,
            prototype: Some(prototype),
            keys,
            values,
        });
        agent[self].object_index = Some(backing_object);
        backing_object
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_prototype(agent)
        } else {
            let intrinsic = match agent[self].kind {
                ExceptionType::Error => ProtoIntrinsics::Error,
                ExceptionType::AggregateError => ProtoIntrinsics::AggregateError,
                ExceptionType::EvalError => ProtoIntrinsics::EvalError,
                ExceptionType::RangeError => ProtoIntrinsics::RangeError,
                ExceptionType::ReferenceError => ProtoIntrinsics::ReferenceError,
                ExceptionType::SyntaxError => ProtoIntrinsics::SyntaxError,
                ExceptionType::TypeError => ProtoIntrinsics::TypeError,
                ExceptionType::UriError => ProtoIntrinsics::UriError,
            };
            Some(
                agent
                    .current_realm()
                    .intrinsics()
                    .get_intrinsic_default_proto(intrinsic),
            )
        }
    }
}

impl InternalMethods for Error {
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<crate::ecmascript::types::PropertyDescriptor>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => backing_object.internal_get_own_property(agent, property_key),
            None => {
                let property_value =
                    if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                        agent[self].message.map(|message| message.into_value())
                    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                        agent[self].cause
                    } else {
                        None
                    };
                Ok(property_value.map(|value| PropertyDescriptor {
                    value: Some(value),
                    writable: Some(true),
                    get: None,
                    set: None,
                    enumerable: Some(false),
                    configurable: Some(true),
                }))
            }
        }
    }

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match self.get_backing_object(agent) {
            Some(backing_object) => backing_object.internal_has_property(agent, property_key),
            None => Ok(
                if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                    agent[self].message.is_some()
                } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                    agent[self].cause.is_some()
                } else {
                    false
                },
            ),
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        match self.get_backing_object(agent) {
            Some(backing_object) => backing_object.internal_get(agent, property_key, receiver),
            None => {
                let property_value =
                    if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                        agent[self].message.map(|message| message.into_value())
                    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                        agent[self].cause
                    } else {
                        None
                    };
                if let Some(property_value) = property_value {
                    Ok(property_value)
                } else if let Some(parent) = self.internal_get_prototype_of(agent)? {
                    // c. Return ? parent.[[Get]](P, Receiver).
                    parent.internal_get(agent, property_key, receiver)
                } else {
                    Ok(Value::Undefined)
                }
            }
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => backing_object.internal_own_property_keys(agent),
            None => {
                let mut property_keys = Vec::with_capacity(2);
                if agent[self].message.is_some() {
                    property_keys.push(BUILTIN_STRING_MEMORY.message.into());
                }
                if agent[self].cause.is_some() {
                    property_keys.push(BUILTIN_STRING_MEMORY.cause.into());
                }
                Ok(property_keys)
            }
        }
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
        &self.heap.errors[index]
    }
}

impl IndexMut<Error> for Agent {
    fn index_mut(&mut self, index: Error) -> &mut Self::Output {
        &mut self.heap.errors[index]
    }
}

impl Index<Error> for Vec<Option<ErrorHeapData>> {
    type Output = ErrorHeapData;

    fn index(&self, index: Error) -> &Self::Output {
        self.get(index.get_index())
            .expect("Error out of bounds")
            .as_ref()
            .expect("Error slot empty")
    }
}

impl IndexMut<Error> for Vec<Option<ErrorHeapData>> {
    fn index_mut(&mut self, index: Error) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Error out of bounds")
            .as_mut()
            .expect("Error slot empty")
    }
}
