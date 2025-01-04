// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Function;
use crate::{
    ecmascript::{
        builtins::ordinary::{
            ordinary_get_own_property, ordinary_own_property_keys, ordinary_set, ordinary_try_set,
        },
        execution::{Agent, JsResult},
        types::{
            language::IntoObject, InternalMethods, InternalSlots, ObjectHeapData, OrdinaryObject,
            PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    engine::{
        context::{GcScope, NoGcScope},
        unwrap_try, TryResult,
    },
    heap::{CreateHeapData, ObjectEntry, ObjectEntryPropertyDescriptor},
};

pub trait IntoFunction<'a>
where
    Self: Sized + Copy + IntoObject,
{
    fn into_function(self) -> Function<'a>;
}

/// Implements getters for the properties normally present on most objects.
/// These are used when the function hasn't had a backing object created.
pub(crate) trait FunctionInternalProperties<'a>
where
    Self: IntoObject + IntoFunction<'a> + InternalSlots + InternalMethods,
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
    let prototype = func.internal_prototype(agent);
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
    let (keys, values) = agent
        .heap
        .elements
        .create_object_entries(&[length_entry, name_entry]);
    let backing_object = agent.heap.create(ObjectHeapData {
        extensible: true,
        prototype,
        keys,
        values,
    });
    func.set_backing_object(agent, backing_object);
    backing_object
}

pub(crate) fn function_internal_get_own_property<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
) -> Option<PropertyDescriptor> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_get_own_property(agent, backing_object, property_key)
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
            value: Some(func.get_name(agent).into_value()),
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
    gc: NoGcScope<'_, '_>,
) -> bool {
    let backing_object = func
        .get_backing_object(agent)
        .unwrap_or_else(|| func.create_backing_object(agent));
    unwrap_try(backing_object.try_define_own_property(agent, property_key, property_descriptor, gc))
}

pub(crate) fn function_try_has_property<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    gc: NoGcScope<'_, '_>,
) -> TryResult<bool> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.try_has_property(agent, property_key, gc)
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

pub(crate) fn function_internal_has_property<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    gc: GcScope<'_, '_>,
) -> JsResult<bool> {
    let property_key = property_key.bind(gc.nogc());
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.internal_has_property(agent, property_key.unbind(), gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        Ok(true)
    } else {
        let parent = unwrap_try(func.try_get_prototype_of(agent, gc.nogc()));
        if let Some(parent) = parent {
            parent.internal_has_property(agent, property_key.unbind(), gc)
        } else {
            Ok(false)
        }
    }
}

pub(crate) fn function_try_get<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    receiver: Value,
    gc: NoGcScope<'_, '_>,
) -> TryResult<Value> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.try_get(agent, property_key, receiver, gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        TryResult::Continue(func.get_length(agent).into())
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        TryResult::Continue(func.get_name(agent).into_value())
    } else {
        let parent = unwrap_try(func.try_get_prototype_of(agent, gc));
        parent.map_or(TryResult::Continue(Value::Undefined), |parent| {
            parent.try_get(agent, property_key, receiver, gc)
        })
    }
}

pub(crate) fn function_internal_get<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    receiver: Value,
    gc: GcScope<'_, '_>,
) -> JsResult<Value> {
    let property_key = property_key.bind(gc.nogc());
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.internal_get(agent, property_key.unbind(), receiver, gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        Ok(func.get_length(agent).into())
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        Ok(func.get_name(agent).into_value())
    } else {
        // Note: Getting a function's prototype never calls JavaScript.
        let parent = unwrap_try(func.try_get_prototype_of(agent, gc.nogc()));
        if let Some(parent) = parent {
            parent.internal_get(agent, property_key.unbind(), receiver, gc)
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
    gc: NoGcScope<'_, '_>,
) -> TryResult<bool> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.try_set(agent, property_key, value, receiver, gc)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        // length and name are not writable
        TryResult::Continue(false)
    } else {
        ordinary_try_set(agent, func.into_object(), property_key, value, receiver, gc)
    }
}

pub(crate) fn function_internal_set<'a>(
    func: impl FunctionInternalProperties<'a>,
    agent: &mut Agent,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    gc: GcScope<'_, '_>,
) -> JsResult<bool> {
    let property_key = property_key.bind(gc.nogc());
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.internal_set(agent, property_key.unbind(), value, receiver, gc)
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
    gc: NoGcScope<'_, '_>,
) -> bool {
    if let Some(backing_object) = func.get_backing_object(agent) {
        unwrap_try(backing_object.try_delete(agent, property_key, gc))
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        let backing_object = func.create_backing_object(agent);
        unwrap_try(backing_object.try_delete(agent, property_key, gc))
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
