// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Function;
use crate::{
    ecmascript::{
        builtins::ordinary::{
            caches::PropertyLookupCache, ordinary_define_own_property, ordinary_delete,
            ordinary_get_own_property, ordinary_has_property, ordinary_own_property_keys,
            ordinary_set, ordinary_try_get, ordinary_try_has_property, ordinary_try_set,
        },
        execution::{
            Agent, JsResult,
            agent::{TryResult, js_result_into_try, unwrap_try},
        },
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, IntoValue, OrdinaryObject,
            PropertyDescriptor, PropertyKey, SetCachedProps, SetResult, String, TryGetResult,
            TryHasResult, Value, language::IntoObject,
        },
    },
    engine::context::{Bindable, GcScope, NoGcScope},
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
        key: BUILTIN_STRING_MEMORY.length.into(),
        value: ObjectEntryPropertyDescriptor::Data {
            value: func.get_length(agent).into(),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    };
    let name_entry = ObjectEntry {
        key: BUILTIN_STRING_MEMORY.name.into(),
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

pub(crate) fn function_set_cached<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    props: &SetCachedProps,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, SetResult<'gc>> {
    let bo = func.get_backing_object(agent);
    if bo.is_none()
        && (props.p == BUILTIN_STRING_MEMORY.length.into()
            || props.p == BUILTIN_STRING_MEMORY.name.into())
    {
        SetResult::Unwritable.into()
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
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> Option<PropertyDescriptor<'gc>> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_get_own_property(
            agent,
            func.into_object().bind(gc),
            backing_object,
            property_key,
            cache,
            gc,
        )
    } else if property_key == BUILTIN_STRING_MEMORY.length.into() {
        Some(PropertyDescriptor {
            value: Some(func.get_length(agent).into()),
            writable: Some(false),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        })
    } else if property_key == BUILTIN_STRING_MEMORY.name.into() {
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

pub(crate) fn function_internal_define_own_property<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    property_descriptor: PropertyDescriptor,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, bool> {
    let backing_object = func
        .get_backing_object(agent)
        .unwrap_or_else(|| func.create_backing_object(agent));
    js_result_into_try(ordinary_define_own_property(
        agent,
        func.into_object(),
        backing_object,
        property_key,
        property_descriptor,
        cache,
        gc,
    ))
}

pub(crate) fn function_try_has_property<'a, 'gc>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, TryHasResult<'gc>> {
    let backing_object = func.get_backing_object(agent);

    if backing_object.is_none()
        && (property_key == BUILTIN_STRING_MEMORY.length.into()
            || property_key == BUILTIN_STRING_MEMORY.name.into())
    {
        let index = if property_key == BUILTIN_STRING_MEMORY.length.into() {
            0
        } else {
            1
        };
        TryHasResult::Custom(index, func.into_object().bind(gc)).into()
    } else {
        ordinary_try_has_property(
            agent,
            func.into_object(),
            backing_object,
            property_key,
            cache,
            gc,
        )
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
    } else if property_key == BUILTIN_STRING_MEMORY.length.into()
        || property_key == BUILTIN_STRING_MEMORY.name.into()
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
) -> TryResult<'gc, TryGetResult<'gc>> {
    let backing_object = func.get_backing_object(agent);
    // if let Some(backing_object) = func.get_backing_object(agent) {
    if backing_object.is_none() && property_key == BUILTIN_STRING_MEMORY.length.into() {
        TryGetResult::Value(func.get_length(agent).into()).into()
    } else if backing_object.is_none() && property_key == BUILTIN_STRING_MEMORY.name.into() {
        TryGetResult::Value(func.get_name(agent).into_value()).into()
    } else {
        ordinary_try_get(
            agent,
            func.into_object(),
            backing_object,
            property_key,
            receiver,
            cache,
            gc,
        )
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
    } else if property_key == BUILTIN_STRING_MEMORY.length.into() {
        Ok(func.get_length(agent).into())
    } else if property_key == BUILTIN_STRING_MEMORY.name.into() {
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

pub(crate) fn function_try_set<'gc, 'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, SetResult<'gc>> {
    if func.get_backing_object(agent).is_some() {
        ordinary_try_set(agent, func, property_key, value, receiver, cache, gc)
    } else if property_key == BUILTIN_STRING_MEMORY.length.into()
        || property_key == BUILTIN_STRING_MEMORY.name.into()
    {
        // length and name are not writable
        SetResult::Unwritable.into()
    } else {
        ordinary_try_set(agent, func, property_key, value, receiver, cache, gc)
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
    } else if property_key == BUILTIN_STRING_MEMORY.length.into()
        || property_key == BUILTIN_STRING_MEMORY.name.into()
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
    } else if property_key == BUILTIN_STRING_MEMORY.length.into()
        || property_key == BUILTIN_STRING_MEMORY.name.into()
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
            BUILTIN_STRING_MEMORY.length.into(),
            BUILTIN_STRING_MEMORY.name.into(),
        ]
    }
}
