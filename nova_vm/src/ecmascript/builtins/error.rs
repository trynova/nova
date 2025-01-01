// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use std::ops::{Index, IndexMut};

pub(crate) use data::ErrorHeapData;

use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::{unwrap_try, TryResult};
use crate::{
    ecmascript::{
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics},
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, ObjectHeapData,
            OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
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
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        let prototype = self.internal_prototype(agent).unwrap();
        let message_entry = agent[self].message.map(|message| ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.message),
            value: ObjectEntryPropertyDescriptor::Data {
                value: message.into_value(),
                writable: true,
                enumerable: false,
                configurable: true,
            },
        });
        let cause_entry = agent[self].cause.map(|cause| ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.cause),
            value: ObjectEntryPropertyDescriptor::Data {
                value: cause,
                writable: true,
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
        self.set_backing_object(agent, backing_object);
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
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<Option<PropertyDescriptor>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => backing_object.try_get_own_property(agent, property_key, gc),
            None => {
                let property_value =
                    if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                        agent[self].message.map(|message| message.into_value())
                    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                        agent[self].cause
                    } else {
                        None
                    };
                TryResult::Continue(property_value.map(|value| PropertyDescriptor {
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

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        match self.get_backing_object(agent) {
            Some(backing_object) => backing_object.try_has_property(agent, property_key, gc),
            None => TryResult::Continue(
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

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        let property_key = property_key.bind(gc.nogc());
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                backing_object.internal_has_property(agent, property_key.unbind(), gc)
            }
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

    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<Value> {
        match self.get_backing_object(agent) {
            Some(backing_object) => backing_object.try_get(agent, property_key, receiver, gc),
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
                    TryResult::Continue(property_value)
                } else if let Some(parent) = unwrap_try(self.try_get_prototype_of(agent, gc)) {
                    // c. Return ? parent.[[Get]](P, Receiver).
                    parent.try_get(agent, property_key, receiver, gc)
                } else {
                    TryResult::Continue(Value::Undefined)
                }
            }
        }
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        let property_key = property_key.bind(gc.nogc());
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                backing_object.internal_get(agent, property_key.unbind(), receiver, gc)
            }
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
                } else if let Some(parent) = unwrap_try(self.try_get_prototype_of(agent, gc.nogc()))
                {
                    // Note: Error is never a prototype so [[GetPrototypeOf]]
                    // cannot call user code.
                    // c. Return ? parent.[[Get]](P, Receiver).
                    parent.internal_get(agent, property_key.unbind(), receiver, gc)
                } else {
                    Ok(Value::Undefined)
                }
            }
        }
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                backing_object.try_set(agent, property_key, value, receiver, gc)
            }
            None => {
                if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
                    && value.is_string()
                {
                    agent[self].message = Some(String::try_from(value).unwrap());
                    TryResult::Continue(true)
                } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                    agent[self].cause = Some(value);
                    TryResult::Continue(true)
                } else {
                    let backing_object = self.create_backing_object(agent);
                    backing_object.try_set(agent, property_key, value, receiver, gc)
                }
            }
        }
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        let property_key = property_key.bind(gc.nogc());
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                backing_object.internal_set(agent, property_key.unbind(), value, receiver, gc)
            }
            None => {
                if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
                    && value.is_string()
                {
                    agent[self].message = Some(String::try_from(value).unwrap());
                    Ok(true)
                } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                    agent[self].cause = Some(value);
                    Ok(true)
                } else {
                    let backing_object = self.create_backing_object(agent);
                    backing_object.internal_set(agent, property_key.unbind(), value, receiver, gc)
                }
            }
        }
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        match self.get_backing_object(agent) {
            Some(backing_object) => TryResult::Continue(unwrap_try(backing_object.try_delete(
                agent,
                property_key,
                gc,
            ))),
            None => {
                if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                    agent[self].message = None;
                } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                    agent[self].cause = None;
                }
                TryResult::Continue(true)
            }
        }
    }

    fn try_own_property_keys<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<Vec<PropertyKey<'a>>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                TryResult::Continue(unwrap_try(backing_object.try_own_property_keys(agent, gc)))
            }
            None => {
                let mut property_keys = Vec::with_capacity(2);
                if agent[self].message.is_some() {
                    property_keys.push(BUILTIN_STRING_MEMORY.message.into());
                }
                if agent[self].cause.is_some() {
                    property_keys.push(BUILTIN_STRING_MEMORY.cause.into());
                }
                TryResult::Continue(property_keys)
            }
        }
    }
}

impl HeapMarkAndSweep for Error {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.errors.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.errors.shift_index(&mut self.0);
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
