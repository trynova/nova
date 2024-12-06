// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Function;
use crate::{
    ecmascript::{
        builtins::ordinary::{ordinary_get_own_property, ordinary_own_property_keys},
        execution::{Agent, JsResult},
        types::{
            language::IntoObject, InternalMethods, InternalSlots, ObjectHeapData, OrdinaryObject,
            PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    engine::context::{GcScope, NoGcScope},
    heap::{CreateHeapData, ObjectEntry, ObjectEntryPropertyDescriptor},
};

pub trait IntoFunction
where
    Self: Sized + Copy + IntoObject,
{
    fn into_function(self) -> Function;
}

/// Implements getters for the properties normally present on most objects.
/// These are used when the function hasn't had a backing object created.
pub(crate) trait FunctionInternalProperties
where
    Self: IntoObject + IntoFunction + InternalSlots + InternalMethods,
{
    /// Value of the 'name' property.
    fn get_name(self, agent: &Agent) -> String;

    /// Value of the 'length' property.
    fn get_length(self, agent: &Agent) -> u8;
}

pub(crate) fn function_create_backing_object(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
) -> OrdinaryObject {
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

pub(crate) fn function_internal_get_own_property(
    func: impl FunctionInternalProperties,
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

pub(crate) fn function_internal_define_own_property(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    gc: NoGcScope<'_, '_>,
    property_key: PropertyKey,
    property_descriptor: PropertyDescriptor,
) -> bool {
    let backing_object = func
        .get_backing_object(agent)
        .unwrap_or_else(|| func.create_backing_object(agent));
    backing_object
        .try_define_own_property(agent, gc, property_key, property_descriptor)
        .unwrap()
}

pub(crate) fn function_try_has_property(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    gc: NoGcScope<'_, '_>,
    property_key: PropertyKey,
) -> Option<bool> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.try_has_property(agent, gc, property_key)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        Some(true)
    } else {
        let parent = func.try_get_prototype_of(agent, gc).unwrap();
        parent.map_or(Some(false), |parent| {
            parent.try_has_property(agent, gc, property_key)
        })
    }
}

pub(crate) fn function_internal_has_property(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    mut gc: GcScope<'_, '_>,
    property_key: PropertyKey,
) -> JsResult<bool> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.internal_has_property(agent, gc.reborrow(), property_key)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        Ok(true)
    } else {
        let parent = func.try_get_prototype_of(agent, gc.nogc()).unwrap();
        parent.map_or(Ok(false), |parent| {
            parent.internal_has_property(agent, gc, property_key)
        })
    }
}

pub(crate) fn function_try_get(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    gc: NoGcScope<'_, '_>,
    property_key: PropertyKey,
    receiver: Value,
) -> Option<Value> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.try_get(agent, gc, property_key, receiver)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        Some(func.get_length(agent).into())
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        Some(func.get_name(agent).into_value())
    } else {
        let parent = func.try_get_prototype_of(agent, gc).unwrap();
        parent.map_or(Some(Value::Undefined), |parent| {
            parent.try_get(agent, gc, property_key, receiver)
        })
    }
}

pub(crate) fn function_internal_get(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    mut gc: GcScope<'_, '_>,
    property_key: PropertyKey,
    receiver: Value,
) -> JsResult<Value> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.internal_get(agent, gc, property_key, receiver)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length) {
        Ok(func.get_length(agent).into())
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name) {
        Ok(func.get_name(agent).into_value())
    } else {
        let parent = func.internal_get_prototype_of(agent, gc.reborrow())?;
        parent.map_or(Ok(Value::Undefined), |parent| {
            parent.internal_get(agent, gc, property_key, receiver)
        })
    }
}

pub(crate) fn function_try_set(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    gc: NoGcScope<'_, '_>,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
) -> Option<bool> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.try_set(agent, gc, property_key, value, receiver)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        // length and name are not writable
        Some(false)
    } else {
        func.create_backing_object(agent)
            .try_set(agent, gc, property_key, value, receiver)
    }
}

pub(crate) fn function_internal_set(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    gc: GcScope<'_, '_>,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
) -> JsResult<bool> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.internal_set(agent, gc, property_key, value, receiver)
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        // length and name are not writable
        Ok(false)
    } else {
        func.create_backing_object(agent)
            .internal_set(agent, gc, property_key, value, receiver)
    }
}

pub(crate) fn function_internal_delete(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    gc: NoGcScope<'_, '_>,
    property_key: PropertyKey,
) -> bool {
    if let Some(backing_object) = func.get_backing_object(agent) {
        backing_object.try_delete(agent, gc, property_key).unwrap()
    } else if property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.length)
        || property_key == PropertyKey::from(BUILTIN_STRING_MEMORY.name)
    {
        let backing_object = func.create_backing_object(agent);
        backing_object.try_delete(agent, gc, property_key).unwrap()
    } else {
        // Non-existing property
        true
    }
}

pub(crate) fn function_internal_own_property_keys<'a>(
    func: impl FunctionInternalProperties,
    agent: &mut Agent,
    gc: NoGcScope<'a, '_>,
) -> Vec<PropertyKey<'a>> {
    if let Some(backing_object) = func.get_backing_object(agent) {
        ordinary_own_property_keys(agent, gc, backing_object)
    } else {
        vec![
            PropertyKey::from(BUILTIN_STRING_MEMORY.length),
            PropertyKey::from(BUILTIN_STRING_MEMORY.name),
        ]
    }
}
