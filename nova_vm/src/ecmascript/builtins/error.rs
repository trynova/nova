// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use core::ops::{Index, IndexMut};
use std::ops::ControlFlow;

pub(crate) use data::ErrorHeapData;

use crate::{
    ecmascript::{
        execution::{Agent, JsResult, ProtoIntrinsics, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, IntoObject, IntoValue, NoCache,
            Object, OrdinaryObject, PropertyDescriptor, PropertyKey, SetCachedProps,
            SetCachedResult, String, TryGetContinue, TryGetResult, TryHasContinue, TryHasResult,
            Value,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
        unwrap_try,
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues, indexes::ErrorIndex,
    },
};

use super::ordinary::{
    caches::PropertyLookupCache, ordinary_delete, ordinary_get_own_property, ordinary_has_property,
    ordinary_set, ordinary_try_get, ordinary_try_has_property, ordinary_try_set,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Error<'a>(pub(crate) ErrorIndex<'a>);

impl Error<'_> {
    pub(crate) const fn _def() -> Self {
        Self(ErrorIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Error<'_> {
    type Of<'a> = Error<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<Error<'a>> for Value<'a> {
    fn from(value: Error<'a>) -> Self {
        Value::Error(value)
    }
}

impl<'a> From<Error<'a>> for Object<'a> {
    fn from(value: Error) -> Self {
        Object::Error(value.unbind())
    }
}

impl<'a> TryFrom<Value<'a>> for Error<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, ()> {
        match value {
            Value::Error(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for Error<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, ()> {
        match value {
            Object::Error(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> InternalSlots<'a> for Error<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Error;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
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
        let backing_object =
            if let (Some(message_entry), Some(cause_entry)) = (message_entry, cause_entry) {
                OrdinaryObject::create_object(agent, Some(prototype), &[message_entry, cause_entry])
            } else if let Some(message_entry) = message_entry {
                OrdinaryObject::create_object(agent, Some(prototype), &[message_entry])
            } else if let Some(cause_entry) = cause_entry {
                OrdinaryObject::create_object(agent, Some(prototype), &[cause_entry])
            } else {
                OrdinaryObject::create_object(agent, Some(prototype), &[])
            };
        self.set_backing_object(agent, backing_object);
        backing_object
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
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
                ExceptionType::UriError => ProtoIntrinsics::URIError,
            };
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .get_intrinsic_default_proto(intrinsic),
            )
        }
    }
}

impl<'a> InternalMethods<'a> for Error<'a> {
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => TryResult::Continue(
                ordinary_get_own_property(
                    agent,
                    self.into_object(),
                    backing_object,
                    property_key,
                    gc,
                )
                .bind(gc),
            ),
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

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryHasResult<'gc> {
        let error = self.bind(gc);
        let backing_object = error.get_backing_object(agent);
        if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
            && agent[error].message.is_some()
        {
            TryHasContinue::Custom(0, error.into_object()).into()
        } else if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause)
            && agent[error].cause.is_some()
        {
            TryHasContinue::Custom(1, error.into_object()).into()
        } else {
            ordinary_try_has_property(
                agent,
                error.into_object(),
                backing_object,
                property_key,
                cache,
                gc,
            )
        }
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_has_property(
                agent,
                self.into_object(),
                backing_object,
                property_key.unbind(),
                gc,
            ),
            None => {
                let found_direct =
                    if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                        agent[self].message.is_some()
                    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                        agent[self].cause.is_some()
                    } else {
                        false
                    };
                if found_direct {
                    Ok(true)
                } else {
                    self.internal_prototype(agent)
                        .unwrap()
                        .internal_has_property(agent, property_key.unbind(), gc)
                }
            }
        }
    }

    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        let backing_object = self.get_backing_object(agent);
        let property_value = if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
        {
            agent[self].message.map(|message| message.into_value())
        } else if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause)
        {
            agent[self].cause
        } else {
            None
        };
        if let Some(property_value) = property_value {
            return TryGetContinue::Value(property_value).into();
        }
        ordinary_try_get(
            agent,
            self.into_object(),
            backing_object,
            property_key,
            receiver,
            cache,
            gc,
        )
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
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
                } else {
                    self.internal_prototype(agent).unwrap().internal_get(
                        agent,
                        property_key.unbind(),
                        receiver,
                        gc,
                    )
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
        gc: NoGcScope,
    ) -> TryResult<bool> {
        if self.get_backing_object(agent).is_some() {
            ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
            && value.is_string()
        {
            agent[self].message = Some(String::try_from(value.unbind()).unwrap());
            TryResult::Continue(true)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
            agent[self].cause = Some(value.unbind());
            TryResult::Continue(true)
        } else {
            self.create_backing_object(agent);
            ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
        }
    }

    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        if self.get_backing_object(agent).is_some() {
            ordinary_set(
                agent,
                self.into_object(),
                property_key.unbind(),
                value,
                receiver,
                gc,
            )
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
            && value.is_string()
        {
            agent[self].message = Some(String::try_from(value.unbind()).unwrap());
            Ok(true)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
            agent[self].cause = Some(value.unbind());
            Ok(true)
        } else {
            self.create_backing_object(agent);
            ordinary_set(
                agent,
                self.into_object(),
                property_key.unbind(),
                value,
                receiver,
                gc,
            )
        }
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self.get_backing_object(agent) {
            Some(backing_object) => TryResult::Continue(ordinary_delete(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                gc,
            )),
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

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
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

    fn set_cached<'gc>(
        self,
        agent: &mut Agent,
        props: &SetCachedProps,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
        if let Some(bo) = self.get_backing_object(agent) {
            bo.set_cached(agent, props, gc)
        } else if props.p == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
            && let Ok(value) = String::try_from(props.value)
        {
            agent[self].message = Some(value.unbind());
            SetCachedResult::Done.into()
        } else if props.p == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
            agent[self].cause = Some(props.value.unbind());
            SetCachedResult::Done.into()
        } else {
            NoCache.into()
        }
    }
}

impl Rootable for Error<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::Error(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Error(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for Error<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.errors.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.errors.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Error<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.errors.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<ErrorHeapData<'a>, Error<'a>> for Heap {
    fn create(&mut self, data: ErrorHeapData<'a>) -> Error<'a> {
        self.errors.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<ErrorHeapData<'static>>>();
        Error(ErrorIndex::last(&self.errors))
    }
}

impl Index<Error<'_>> for Agent {
    type Output = ErrorHeapData<'static>;

    fn index(&self, index: Error) -> &Self::Output {
        &self.heap.errors[index]
    }
}

impl IndexMut<Error<'_>> for Agent {
    fn index_mut(&mut self, index: Error) -> &mut Self::Output {
        &mut self.heap.errors[index]
    }
}

impl Index<Error<'_>> for Vec<Option<ErrorHeapData<'static>>> {
    type Output = ErrorHeapData<'static>;

    fn index(&self, index: Error) -> &Self::Output {
        self.get(index.get_index())
            .expect("Error out of bounds")
            .as_ref()
            .expect("Error slot empty")
    }
}

impl IndexMut<Error<'_>> for Vec<Option<ErrorHeapData<'static>>> {
    fn index_mut(&mut self, index: Error) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Error out of bounds")
            .as_mut()
            .expect("Error slot empty")
    }
}
