// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{collections::TryReserveError, ops::ControlFlow};

use super::Function;
use crate::{
    ecmascript::{
        builtins::ordinary::{
            caches::PropertyLookupCache, ordinary_define_own_property, ordinary_delete,
            ordinary_get_own_property, ordinary_has_property, ordinary_own_property_keys,
            ordinary_set, ordinary_try_get, ordinary_try_has_property, ordinary_try_set,
        },
        execution::{Agent, JsResult},
        types::{
            BUILTIN_STRING_MEMORY, GetCachedResult, InternalMethods, InternalSlots, IntoValue,
            NoCache, OrdinaryObject, PropertyDescriptor, PropertyKey, SetCachedProps,
            SetCachedResult, String, Value, language::IntoObject,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
        unwrap_try,
    },
    heap::{ObjectEntry, ObjectEntryPropertyDescriptor},
};

pub trait IntoFunction<'a>
where
    Self: 'a + Sized + Copy + IntoObject<'a>,
{
    fn into_function(self) -> Function<'a>;
}

impl<'a, T> IntoFunction<'a> for T
where
    T: Into<Function<'a>> + 'a + Sized + Copy + IntoObject<'a>,
{
    #[inline]
    fn into_function(self) -> Function<'a> {
        self.into()
    }
}

/// Implements getters for the properties normally present on most objects.
/// These are used when the function hasn't had a backing object created.
pub(crate) trait FunctionInternalProperties<'a>
where
    Self: IntoObject<'a> + IntoFunction<'a> + InternalSlots<'a> + InternalMethods<'a>,
{
    /// Value of the 'name' property.
    fn get_name(self, agent: &Agent) -> String<'static>;

    /// Value of the 'length' property.
    fn get_length(self, agent: &Agent) -> u8;
}

pub(crate) fn function_create_backing_object<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
) -> OrdinaryObject<'static> {
    assert!(func.get_backing_object(agent).is_none());
    let prototype = func.internal_prototype(agent).unwrap();
    let length_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
        value: ObjectEntryPropertyDescriptor::Data {
            value: func.get_length(agent).into(),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    };
    let name_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
        value: ObjectEntryPropertyDescriptor::Data {
            value: func.get_name(agent).into_value(),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    };
    let backing_object =
        OrdinaryObject::create_object(agent, Some(prototype), &[length_entry, name_entry]);
    func.set_backing_object(agent, backing_object);
    backing_object
}

pub(crate) fn function_get_cached<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    p: PropertyKey,
    cache: PropertyLookupCache,
    gc: NoGcScope<'gc, '_>,
) -> ControlFlow<GetCachedResult<'gc>, NoCache> {
    let bo = func.get_backing_object(agent);
    if bo.is_none() && p == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        func.get_length(agent).into_value().bind(gc).into()
    } else if bo.is_none() && p == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        func.get_name(agent).into_value().bind(gc).into()
    } else {
        let shape = if let Some(bo) = bo {
            bo.object_shape(agent)
        } else {
            func.object_shape(agent)
        };
        shape.get_cached(agent, p, func.into_value(), cache, gc)
    }
}

pub(crate) fn function_set_cached<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    props: &SetCachedProps,
    gc: NoGcScope<'gc, '_>,
) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
    let bo = func.get_backing_object(agent);
    if bo.is_none()
        && (props.p == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
            || props.p == PropertyKey::from(BUILTIN_STRING_MEMORY.name))
    {
        SetCachedResult::Unwritable.into()
    } else {
        let shape = if let Some(bo) = bo {
            bo.object_shape(agent)
        } else {
            func.object_shape(agent)
        };
        shape.set_cached(agent, func.into_object(), props, gc)
    }
}

pub(crate) fn function_internal_get_own_property<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    gc: NoGcScope<'gc, '_>,
) -> Option<PropertyDescriptor<'gc>> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_get_own_property(
            agent,
            func.into_object().bind(gc),
            backing_object,
            property_key,
            gc,
        )
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        Some(PropertyDescriptor {
            value: Some(func.get_length(agent).into()),
            writable: Some(false),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        })
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        Some(PropertyDescriptor {
            value: Some(func.get_name(agent).into_value().bind(gc)),
            writable: Some(false),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        })
    } else {
        None
    }
}

pub(crate) fn function_internal_define_own_property<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    property_descriptor: PropertyDescriptor,
    gc: NoGcScope,
) -> Result<bool, TryReserveError> {
    let backing_object = func
        .get_backing_object(agent)
        .unwrap_or_else(|| func.create_backing_object(agent));
    ordinary_define_own_property(
        agent,
        func.into_object(),
        backing_object,
        property_key,
        property_descriptor,
        gc,
    )
}

pub(crate) fn function_try_has_property<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    gc: NoGcScope,
) -> TryResult<bool> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_try_has_property(agent, func.into_object(), backing_object, property_key, gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        TryResult::Continue(true)
    } else {
        let parent = unwrap_try(func.try_get_prototype_of(agent, gc));
        parent.map_or(TryResult::Continue(false), |parent| {
            parent.try_has_property(agent, property_key, gc)
        })
    }
}

pub(crate) fn function_internal_has_property<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    let property_key = property_key.bind(gc.nogc());
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_has_property(
            agent,
            func.into_object(),
            backing_object,
            property_key.unbind(),
            gc,
        )
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        Ok(true)
    } else {
        let parent = unwrap_try(func.try_get_prototype_of(agent, gc.nogc()));
        if let Some(parent) = parent {
            parent
                .unbind()
                .internal_has_property(agent, property_key.unbind(), gc)
        } else {
            Ok(false)
        }
    }
}

pub(crate) fn function_try_get<'gc, 'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    receiver: Value,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<Value<'gc>> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_try_get(
            agent,
            func.into_object(),
            backing_object,
            property_key,
            receiver,
            gc,
        )
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        TryResult::Continue(func.get_length(agent).into())
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        TryResult::Continue(func.get_name(agent).into_value())
    } else {
        let parent = unwrap_try(func.try_get_prototype_of(agent, gc));
        parent.map_or(TryResult::Continue(Value::Undefined), |parent| {
            parent.try_get(agent, property_key, receiver, None, gc)
        })
    }
}

pub(crate) fn function_internal_get<'gc, 'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    receiver: Value,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let property_key = property_key.bind(gc.nogc());
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.internal_get(agent, property_key.unbind(), receiver, gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        Ok(func.get_length(agent).into())
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        Ok(func.get_name(agent).into_value().bind(gc.into_nogc()))
    } else {
        // Note: Getting a function's prototype never calls JavaScript.
        let parent = unwrap_try(func.try_get_prototype_of(agent, gc.nogc()));
        if let Some(parent) = parent {
            parent
                .unbind()
                .internal_get(agent, property_key.unbind(), receiver, gc)
        } else {
            Ok(Value::Undefined)
        }
    }
}

pub(crate) fn function_try_set<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    gc: NoGcScope,
) -> TryResult<bool> {
    if func.get_backing_object(agent).is_some() {
        ordinary_try_set(agent, func.into_object(), property_key, value, receiver, gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        // length and name are not writable
        TryResult::Continue(false)
    } else {
        ordinary_try_set(agent, func.into_object(), property_key, value, receiver, gc)
    }
}

pub(crate) fn function_internal_set<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    let property_key = property_key.bind(gc.nogc());
    if func.get_backing_object(agent).is_some() {
        ordinary_set(
            agent,
            func.into_object(),
            property_key.unbind(),
            value,
            receiver,
            gc,
        )
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        // length and name are not writable
        Ok(false)
    } else {
        ordinary_set(
            agent,
            func.into_object(),
            property_key.unbind(),
            value,
            receiver,
            gc,
        )
    }
}

pub(crate) fn function_internal_delete<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    gc: NoGcScope,
) -> bool {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_delete(agent, func.into_object(), backing_object, property_key, gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        let backing_object = func.create_backing_object(agent);
        ordinary_delete(agent, func.into_object(), backing_object, property_key, gc)
    } else {
        // Non-existing property
        true
    }
}

pub(crate) fn function_internal_own_property_keys<'a, 'b>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    gc: NoGcScope<'b, '_>,
) -> Vec<PropertyKey<'b>> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_own_property_keys(agent, backing_object, gc)
    } else {
        vec![
            PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            PropertyKey::from(BUILTIN_STRING_MEMORY.name),
        ]
    }
}
