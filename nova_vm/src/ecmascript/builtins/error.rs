// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub(crate) use data::*;

use crate::{
    ecmascript::{
        Agent, BUILTIN_STRING_MEMORY, ExceptionType, InternalMethods, InternalSlots, JsResult,
        Object, OrdinaryObject, PropertyDescriptor, PropertyKey, ProtoIntrinsics, SetResult,
        String, TryGetResult, TryHasResult, TryResult, Value, object_handle, unwrap_try,
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::{
        ArenaAccess, ArenaAccessMut, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues,
        arena_vec_access, indexes::BaseIndex,
    },
};

use super::ordinary::{
    PropertyLookupCache, ordinary_delete, ordinary_get_own_property, ordinary_has_property,
    ordinary_set, ordinary_try_get, ordinary_try_has_property, ordinary_try_set,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Error<'a>(BaseIndex<'a, ErrorHeapData<'static>>);
object_handle!(Error);
arena_vec_access!(
    Error,
    'a,
    ErrorHeapData,
    errors
);

impl<'a> InternalSlots<'a> for Error<'a> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Error;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        let prototype = self.internal_prototype(agent).unwrap();
        let message_entry = self.get(agent).message.map(|message| ObjectEntry {
            key: PropertyKey::from(BUILTIN_STRING_MEMORY.message),
            value: ObjectEntryPropertyDescriptor::Data {
                value: message.into(),
                writable: true,
                enumerable: false,
                configurable: true,
            },
        });
        let cause_entry = self.get(agent).cause.map(|cause| ObjectEntry {
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
            }
            .expect("Should perform GC here")
            .unbind();
        self.set_backing_object(agent, backing_object);
        backing_object
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        if let Some(object_index) = self.get_backing_object(agent) {
            object_index.internal_prototype(agent)
        } else {
            let intrinsic = match self.get(agent).kind {
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
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => TryResult::Continue(
                ordinary_get_own_property(
                    agent,
                    self.into(),
                    backing_object,
                    property_key,
                    cache,
                    gc,
                )
                .bind(gc),
            ),
            None => {
                let property_value =
                    if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                        self.get(agent).message.map(|message| message.into())
                    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                        self.get(agent).cause
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
                .unbind()
            }
        }
    }

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        let error = self.bind(gc);
        let backing_object = error.get_backing_object(agent);
        if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
            && error.get(agent).message.is_some()
        {
            TryHasResult::Custom(0, error.into()).into()
        } else if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause)
            && error.get(agent).cause.is_some()
        {
            TryHasResult::Custom(1, error.into()).into()
        } else {
            ordinary_try_has_property(agent, error.into(), backing_object, property_key, cache, gc)
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
                self.into(),
                backing_object,
                property_key.unbind(),
                gc,
            ),
            None => {
                let found_direct =
                    if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                        self.get(agent).message.is_some()
                    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                        self.get(agent).cause.is_some()
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
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        let backing_object = self.get_backing_object(agent);
        let property_value = if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
        {
            self.get(agent).message.map(|message| message.into())
        } else if backing_object.is_none()
            && property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause)
        {
            self.get(agent).cause
        } else {
            None
        };
        if let Some(property_value) = property_value {
            return TryGetResult::Value(property_value.unbind()).into();
        }
        ordinary_try_get(
            agent,
            self.into(),
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
                        self.get(agent).message.map(|message| message.into())
                    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                        self.get(agent).cause
                    } else {
                        None
                    };
                if let Some(property_value) = property_value {
                    Ok(property_value.unbind())
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

    fn try_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        if self.get_backing_object(agent).is_some() {
            ordinary_try_set(agent, self, property_key, value, receiver, cache, gc)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
            && value.is_string()
        {
            self.get_mut(agent).message = Some(String::try_from(value.unbind()).unwrap());
            SetResult::Done.into()
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
            self.get_mut(agent).cause = Some(value.unbind());
            SetResult::Done.into()
        } else {
            self.create_backing_object(agent);
            ordinary_try_set(agent, self, property_key, value, receiver, cache, gc)
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
                self.into(),
                property_key.unbind(),
                value,
                receiver,
                gc,
            )
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message)
            && value.is_string()
        {
            self.get_mut(agent).message = Some(String::try_from(value.unbind()).unwrap());
            Ok(true)
        } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
            self.get_mut(agent).cause = Some(value.unbind());
            Ok(true)
        } else {
            self.create_backing_object(agent);
            ordinary_set(
                agent,
                self.into(),
                property_key.unbind(),
                value,
                receiver,
                gc,
            )
        }
    }

    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match self.get_backing_object(agent) {
            Some(backing_object) => TryResult::Continue(ordinary_delete(
                agent,
                self.into(),
                backing_object,
                property_key,
                gc,
            )),
            None => {
                if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.message) {
                    self.get_mut(agent).message = None;
                } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.cause) {
                    self.get_mut(agent).cause = None;
                }
                TryResult::Continue(true)
            }
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                TryResult::Continue(unwrap_try(backing_object.try_own_property_keys(agent, gc)))
            }
            None => {
                let mut property_keys = Vec::with_capacity(2);
                if self.get_mut(agent).message.is_some() {
                    property_keys.push(BUILTIN_STRING_MEMORY.message.into());
                }
                if self.get_mut(agent).cause.is_some() {
                    property_keys.push(BUILTIN_STRING_MEMORY.cause.into());
                }
                TryResult::Continue(property_keys)
            }
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
        self.errors.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<ErrorHeapData<'static>>();
        Error(BaseIndex::last(&self.errors))
    }
}
