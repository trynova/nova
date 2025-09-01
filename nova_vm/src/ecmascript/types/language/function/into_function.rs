// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::Function;
use crate::{
    ecmascript::{
        builtins::{
            ArgumentsList,
            ordinary::{
                caches::PropertyLookupCache, ordinary_define_own_property, ordinary_delete,
                ordinary_get_own_property, ordinary_has_property, ordinary_own_property_keys,
                ordinary_set, ordinary_try_get, ordinary_try_has_property, ordinary_try_set,
            },
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics,
            agent::{TryResult, js_result_into_try, unwrap_try},
        },
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, IntoValue, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, SetResult, String, TryGetResult,
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
    Self: Sized + Copy + Into<Object<'a>> + IntoObject<'a> + IntoFunction<'a> + core::fmt::Debug,
{
    /// Value of the 'name' property.
    fn get_name(self, agent: &Agent) -> String<'static>;

    /// Value of the 'length' property.
    fn get_length(self, agent: &Agent) -> u8;

    fn get_function_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>>;

    fn set_function_backing_object(
        self,
        agent: &mut Agent,
        backing_object: OrdinaryObject<'static>,
    );

    // #### \[\[Prototype\]\]
    fn function_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_prototype(agent)
        } else {
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .get_intrinsic_default_proto(Self::DEFAULT_PROTOTYPE),
            )
        }
    }

    #[allow(unused_variables)]
    fn function_call<'gc>(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>>;

    #[allow(unused_variables)]
    fn function_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        unreachable!()
    }
}

impl<'a, T: 'a + FunctionInternalProperties<'a>> InternalSlots<'a> for T {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get_function_backing_object(agent)
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        self.set_function_backing_object(agent, backing_object)
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        assert!(self.get_backing_object(agent).is_none());
        let prototype = self.internal_prototype(agent).unwrap();
        let length_entry = ObjectEntry {
            key: BUILTIN_STRING_MEMORY.length.into(),
            value: ObjectEntryPropertyDescriptor::Data {
                value: self.get_length(agent).into(),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        };
        let name_entry = ObjectEntry {
            key: BUILTIN_STRING_MEMORY.name.into(),
            value: ObjectEntryPropertyDescriptor::Data {
                value: self.get_name(agent).into_value(),
                writable: false,
                enumerable: false,
                configurable: true,
            },
        };
        let backing_object =
            OrdinaryObject::create_object(agent, Some(prototype), &[length_entry, name_entry]);
        self.set_backing_object(agent, backing_object);
        backing_object
    }

    fn internal_prototype(self, agent: &Agent) -> Option<Object<'static>> {
        self.function_prototype(agent)
    }
}

impl<'a, T: 'a + FunctionInternalProperties<'a>> InternalMethods<'a> for T {
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            TryResult::Continue(ordinary_get_own_property(
                agent,
                self.into_object().bind(gc),
                backing_object,
                property_key,
                cache,
                gc,
            ))
        } else if property_key == BUILTIN_STRING_MEMORY.length.into() {
            TryResult::Continue(Some(PropertyDescriptor {
                value: Some(self.get_length(agent).into()),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
        } else if property_key == BUILTIN_STRING_MEMORY.name.into() {
            TryResult::Continue(Some(PropertyDescriptor {
                value: Some(self.get_name(agent).into_value().bind(gc)),
                writable: Some(false),
                enumerable: Some(false),
                configurable: Some(true),
                ..Default::default()
            }))
        } else {
            TryResult::Continue(None)
        }
    }

    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        js_result_into_try(ordinary_define_own_property(
            agent,
            self.into_object(),
            backing_object,
            property_key,
            property_descriptor,
            cache,
            gc,
        ))
    }

    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        let backing_object = self.get_backing_object(agent);

        if backing_object.is_none()
            && (property_key == BUILTIN_STRING_MEMORY.length.into()
                || property_key == BUILTIN_STRING_MEMORY.name.into())
        {
            let index = if property_key == BUILTIN_STRING_MEMORY.length.into() {
                0
            } else {
                1
            };
            TryHasResult::Custom(index, self.into_object().bind(gc)).into()
        } else {
            ordinary_try_has_property(
                agent,
                self.into_object(),
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
        if let Some(backing_object) = self.get_backing_object(agent) {
            ordinary_has_property(
                agent,
                self.into_object(),
                backing_object,
                property_key.unbind(),
                gc,
            )
        } else if property_key == BUILTIN_STRING_MEMORY.length.into()
            || property_key == BUILTIN_STRING_MEMORY.name.into()
        {
            Ok(true)
        } else {
            let parent = unwrap_try(self.try_get_prototype_of(agent, gc.nogc()));
            if let Some(parent) = parent {
                parent
                    .unbind()
                    .internal_has_property(agent, property_key.unbind(), gc)
            } else {
                Ok(false)
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
        // if let Some(backing_object) = self.get_backing_object(agent) {
        if backing_object.is_none() && property_key == BUILTIN_STRING_MEMORY.length.into() {
            TryGetResult::Value(self.get_length(agent).into()).into()
        } else if backing_object.is_none() && property_key == BUILTIN_STRING_MEMORY.name.into() {
            TryGetResult::Value(self.get_name(agent).into_value()).into()
        } else {
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
    }

    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let property_key = property_key.bind(gc.nogc());
        if let Some(backing_object) = self.get_backing_object(agent) {
            backing_object.internal_get(agent, property_key.unbind(), receiver, gc)
        } else if property_key == BUILTIN_STRING_MEMORY.length.into() {
            Ok(self.get_length(agent).into())
        } else if property_key == BUILTIN_STRING_MEMORY.name.into() {
            Ok(self.get_name(agent).into_value().bind(gc.into_nogc()))
        } else {
            // Note: Getting a function's prototype never calls JavaScript.
            let parent = unwrap_try(self.try_get_prototype_of(agent, gc.nogc()));
            if let Some(parent) = parent {
                parent
                    .unbind()
                    .internal_get(agent, property_key.unbind(), receiver, gc)
            } else {
                Ok(Value::Undefined)
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
        } else if property_key == BUILTIN_STRING_MEMORY.length.into()
            || property_key == BUILTIN_STRING_MEMORY.name.into()
        {
            // length and name are not writable
            SetResult::Unwritable.into()
        } else {
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
                self.into_object(),
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
                self.into_object(),
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
        if let Some(backing_object) = self.get_backing_object(agent) {
            TryResult::Continue(ordinary_delete(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                gc,
            ))
        } else if property_key == BUILTIN_STRING_MEMORY.length.into()
            || property_key == BUILTIN_STRING_MEMORY.name.into()
        {
            let backing_object = self.create_backing_object(agent);
            TryResult::Continue(ordinary_delete(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                gc,
            ))
        } else {
            // Non-existing property
            TryResult::Continue(true)
        }
    }

    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        if let Some(backing_object) = self.get_backing_object(agent) {
            TryResult::Continue(ordinary_own_property_keys(agent, backing_object, gc))
        } else {
            TryResult::Continue(vec![
                BUILTIN_STRING_MEMORY.length.into(),
                BUILTIN_STRING_MEMORY.name.into(),
            ])
        }
    }

    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        self.function_call(agent, this_value, arguments_list, gc)
    }

    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        self.function_construct(agent, arguments_list, new_target, gc)
    }
}
