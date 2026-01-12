// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::ControlFlow;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{get, has_property, try_get, try_has_property},
            testing_and_comparison::is_callable,
            type_conversion::to_boolean,
        },
        builtins::ordinary::caches::PropertyLookupCache,
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, TryError, TryResult},
        },
        types::{
            BUILTIN_STRING_MEMORY, Function, Object, OrdinaryObject, TryHasResult, Value,
            try_get_result_into_value,
        },
    },
    engine::{
        Scoped,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::Scopable,
    },
    heap::ObjectEntry,
};

/// ### [6.2.6 The Property Descriptor Specification Type](https://tc39.es/ecma262/#sec-property-descriptor-specification-type)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PropertyDescriptor<'a> {
    /// \[\[Value]]
    pub value: Option<Value<'a>>,

    /// \[\[Writable]]
    pub writable: Option<bool>,

    /// \[\[Get]]
    ///
    /// Note: double-Option because these fields can be set explicitly undefined
    /// to unset them.
    pub get: Option<Option<Function<'a>>>,

    /// \[\[Set]]
    ///
    /// Note: double-Option because these fields can be set explicitly undefined
    /// to unset them.
    pub set: Option<Option<Function<'a>>>,

    /// \[\[Enumerable]]
    pub enumerable: Option<bool>,

    /// \[\[Configurable]]
    pub configurable: Option<bool>,
}

#[derive(Debug)]
pub struct ScopedPropertyDescriptor<'a> {
    /// \[\[Value]]
    pub value: Option<Scoped<'a, Value<'static>>>,

    /// \[\[Writable]]
    pub writable: Option<bool>,

    /// \[\[Get]]
    ///
    /// Note: double-Option because these fields can be set explicitly undefined
    /// to unset them.
    pub get: Option<Option<Scoped<'a, Function<'static>>>>,

    /// \[\[Set]]
    ///
    /// Note: double-Option because these fields can be set explicitly undefined
    /// to unset them.
    pub set: Option<Option<Scoped<'a, Function<'static>>>>,

    /// \[\[Enumerable]]
    pub enumerable: Option<bool>,

    /// \[\[Configurable]]
    pub configurable: Option<bool>,
}

impl<'b> ScopedPropertyDescriptor<'b> {
    /// Return the property descriptor as unscoped.
    pub(crate) fn get<'a>(&self, agent: &Agent, gc: NoGcScope<'a, 'b>) -> PropertyDescriptor<'a> {
        PropertyDescriptor {
            value: self.value.as_ref().map(|v| v.get(agent).bind(gc)),
            writable: self.writable,
            get: self
                .get
                .as_ref()
                .map(|f| f.as_ref().map(|f| f.get(agent).bind(gc))),
            set: self
                .set
                .as_ref()
                .map(|f| f.as_ref().map(|f| f.get(agent).bind(gc))),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    /// Take ownership of the scoped property descriptor and return it as an
    /// unscoped property descriptor.
    pub(crate) fn take<'a>(self, agent: &Agent, gc: NoGcScope<'a, 'b>) -> PropertyDescriptor<'a> {
        PropertyDescriptor {
            // SAFETY: PropertyDescriptor cannot be shared.
            value: self.value.map(|v| unsafe { v.take(agent).bind(gc) }),
            writable: self.writable,
            // SAFETY: PropertyDescriptor cannot be shared.
            get: self
                .get
                .map(|f| f.map(|f| unsafe { f.take(agent).bind(gc) })),
            // SAFETY: PropertyDescriptor cannot be shared.
            set: self
                .set
                .map(|f| f.map(|f| unsafe { f.take(agent).bind(gc) })),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }
}

impl<'a> PropertyDescriptor<'a> {
    pub(crate) fn scope<'b>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'b>,
    ) -> ScopedPropertyDescriptor<'b> {
        ScopedPropertyDescriptor {
            value: self.value.map(|v| v.scope(agent, gc)),
            writable: self.writable,
            get: self.get.map(|f| f.map(|f| f.scope(agent, gc))),
            set: self.set.map(|f| f.map(|f| f.scope(agent, gc))),
            enumerable: self.enumerable,
            configurable: self.configurable,
        }
    }

    pub fn new_data_descriptor(value: impl Into<Value<'a>>) -> Self {
        Self {
            value: Some(value.into()),
            writable: Some(true),
            get: None,
            set: None,
            enumerable: Some(true),
            configurable: Some(true),
        }
    }

    pub fn new_prototype_method_descriptor(function: impl Into<Function<'a>>) -> Self {
        Self {
            value: Some(function.into().unbind().into()),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..Default::default()
        }
    }

    /// ### [6.2.6.1 IsAccessorDescriptor ( Desc )](https://tc39.es/ecma262/#sec-isaccessordescriptor)
    pub fn is_accessor_descriptor(&self) -> bool {
        // 1. If Desc is undefined, return false.
        match (self.get, self.set) {
            // 2. If Desc has a [[Get]] field, return true.
            (Some(_), _) => true,
            // 3. If Desc has a [[Set]] field, return true.
            (_, Some(_)) => true,
            // 4. Return false.
            _ => false,
        }
    }

    /// ### [6.2.6.2 IsDataDescriptor ( Desc )](https://tc39.es/ecma262/#sec-isdatadescriptor)
    pub fn is_data_descriptor(&self) -> bool {
        // 1. If Desc is undefined, return false.
        match (self.value, self.writable) {
            // 2. If Desc has a [[Value]] field, return true.
            (Some(_), _) => true,
            // 3. If Desc has a [[Writable]] field, return true.
            (_, Some(_)) => true,
            // 4. Return false.
            _ => false,
        }
    }

    /// ### [6.2.6.3 IsGenericDescriptor ( Desc )](https://tc39.es/ecma262/#sec-isgenericdescriptor)
    pub fn is_generic_descriptor(&self) -> bool {
        // 1. If Desc is undefined, return false.
        // 2. If IsAccessorDescriptor(Desc) is true, return false.
        // 3. If IsDataDescriptor(Desc) is true, return false.
        // 4. Return true.
        !self.is_accessor_descriptor() && !self.is_data_descriptor()
    }

    /// ### [6.2.6.4 FromPropertyDescriptor ( Desc )](https://tc39.es/ecma262/#sec-frompropertydescriptor)
    ///
    /// The abstract operation FromPropertyDescriptor takes argument Desc (a
    /// Property Descriptor or undefined) and returns an Object or undefined.
    #[allow(unknown_lints, agent_comes_first)]
    pub fn from_property_descriptor(
        desc: Option<Self>,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> Option<OrdinaryObject<'a>> {
        // 1. If Desc is undefined, return undefined.
        let desc = desc?;

        let mut entries = Vec::with_capacity(4);

        // 4. If Desc has a [[Value]] field, then
        if let Some(value) = desc.value {
            // a. Perform ! CreateDataPropertyOrThrow(obj, "value", Desc.[[Value]]).
            entries.push(ObjectEntry::new_data_entry(
                BUILTIN_STRING_MEMORY.value.into(),
                value,
            ));
        }

        // 5. If Desc has a [[Writable]] field, then
        if let Some(writable) = desc.writable {
            // a. Perform ! CreateDataPropertyOrThrow(obj, "writable", Desc.[[Writable]]).
            entries.push(ObjectEntry::new_data_entry(
                BUILTIN_STRING_MEMORY.writable.into(),
                writable.into(),
            ));
        }

        // 6. If Desc has a [[Get]] field, then
        if let Some(get) = desc.get {
            // a. Perform ! CreateDataPropertyOrThrow(obj, "get", Desc.[[Get]]).
            entries.push(ObjectEntry::new_data_entry(
                BUILTIN_STRING_MEMORY.get.into(),
                get.into(),
            ));
        }

        // 7. If Desc has a [[Set]] field, then
        if let Some(set) = desc.set {
            // a. Perform ! CreateDataPropertyOrThrow(obj, "set", Desc.[[Set]]).
            entries.push(ObjectEntry::new_data_entry(
                BUILTIN_STRING_MEMORY.set.into(),
                set.into(),
            ));
        }

        // 8. If Desc has an [[Enumerable]] field, then
        if let Some(enumerable) = desc.enumerable {
            // a. Perform ! CreateDataPropertyOrThrow(obj, "enumerable", Desc.[[Enumerable]]).
            entries.push(ObjectEntry::new_data_entry(
                BUILTIN_STRING_MEMORY.enumerable.into(),
                enumerable.into(),
            ));
        }

        // 9. If Desc has a [[Configurable]] field, then
        if let Some(configurable) = desc.configurable {
            // a. Perform ! CreateDataPropertyOrThrow(obj, "configurable", Desc.[[Configurable]]).
            entries.push(ObjectEntry::new_data_entry(
                BUILTIN_STRING_MEMORY.configurable.into(),
                configurable.into(),
            ));
        }

        debug_assert!(entries.len() <= 4);

        // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        // 3. Assert: obj is an extensible ordinary object with no own properties.
        let obj = OrdinaryObject::create_object(
            agent,
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into(),
            ),
            &entries,
        )
        .expect("Should perform GC here");

        // 10. Return obj.
        Some(obj.bind(gc))
    }

    /// ### [6.2.6.5 ToPropertyDescriptor ( Obj )](https://tc39.es/ecma262/#sec-topropertydescriptor)
    ///
    /// The abstract operation ToPropertyDescriptor takes argument Obj (an
    /// ECMAScript language value) and returns either a normal completion
    /// containing a Property Descriptor or a throw completion.
    pub fn to_property_descriptor(
        agent: &mut Agent,
        obj: Value,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Self> {
        let obj = obj.bind(gc.nogc());

        // 1. If Obj is not an Object, throw a TypeError exception.
        let Ok(obj) = Object::try_from(obj) else {
            let obj_repr = obj.unbind().string_repr(agent, gc.reborrow());
            let error_message = format!(
                "Property descriptor must be an object, got '{}'.",
                obj_repr.to_string_lossy_(agent)
            );
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                error_message,
                gc.into_nogc(),
            ));
        };
        let scoped_obj = obj.scope(agent, gc.nogc());

        // 2. Let desc be a new Property Descriptor that initially has no
        // fields.
        let mut desc = PropertyDescriptor::default();
        // 3. Let hasEnumerable be ? HasProperty(Obj, "enumerable").
        let has_enumerable = has_property(
            agent,
            obj.unbind(),
            BUILTIN_STRING_MEMORY.enumerable.into(),
            gc.reborrow(),
        )
        .unbind()?;
        // 4. If hasEnumerable is true, then
        if has_enumerable {
            // a. Let enumerable be ToBoolean(? Get(Obj, "enumerable")).
            let enumerable = get(
                agent,
                scoped_obj.get(agent),
                BUILTIN_STRING_MEMORY.enumerable.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let enumerable = to_boolean(agent, enumerable);
            // b. Set desc.[[Enumerable]] to enumerable.
            desc.enumerable = Some(enumerable);
        }
        // 5. Let hasConfigurable be ? HasProperty(Obj, "configurable").
        let has_configurable = has_property(
            agent,
            scoped_obj.get(agent),
            BUILTIN_STRING_MEMORY.configurable.into(),
            gc.reborrow(),
        )
        .unbind()?;
        // 6. If hasConfigurable is true, then
        if has_configurable {
            // a. Let configurable be ToBoolean(? Get(Obj, "configurable")).
            let configurable = get(
                agent,
                scoped_obj.get(agent),
                BUILTIN_STRING_MEMORY.configurable.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let configurable = to_boolean(agent, configurable);
            // b. Set desc.[[Configurable]] to configurable.
            desc.configurable = Some(configurable);
        }
        // 7. Let hasValue be ? HasProperty(Obj, "value").
        let has_value = has_property(
            agent,
            scoped_obj.get(agent),
            BUILTIN_STRING_MEMORY.value.into(),
            gc.reborrow(),
        )
        .unbind()?;
        // 8. If hasValue is true, then
        if has_value {
            // a. Let value be ? Get(Obj, "value").
            let value = get(
                agent,
                scoped_obj.get(agent),
                BUILTIN_STRING_MEMORY.value.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. Set desc.[[Value]] to value.
            desc.value = Some(value.unbind());
        }
        // 9. Let hasWritable be ? HasProperty(Obj, "writable").
        let has_writable = has_property(
            agent,
            scoped_obj.get(agent),
            BUILTIN_STRING_MEMORY.writable.into(),
            gc.reborrow(),
        )
        .unbind()?;
        // 10. If hasWritable is true, then
        if has_writable {
            // a. Let writable be ToBoolean(? Get(Obj, "writable")).
            let writable = get(
                agent,
                scoped_obj.get(agent),
                BUILTIN_STRING_MEMORY.writable.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let writable = to_boolean(agent, writable);
            // b. Set desc.[[Writable]] to writable.
            desc.writable = Some(writable);
        }
        // 11. Let hasGet be ? HasProperty(Obj, "get").
        let has_get = has_property(
            agent,
            scoped_obj.get(agent),
            BUILTIN_STRING_MEMORY.get.into(),
            gc.reborrow(),
        )
        .unbind()?;
        // 12. If hasGet is true, then
        if has_get {
            // a. Let getter be ? Get(Obj, "get").
            let getter = get(
                agent,
                scoped_obj.get(agent),
                BUILTIN_STRING_MEMORY.get.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. If IsCallable(getter) is false and getter is not undefined,
            // throw a TypeError exception.
            if !getter.is_undefined() {
                let Some(getter) = is_callable(getter, gc.nogc()) else {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "getter is not callable",
                        gc.into_nogc(),
                    ));
                };
                // c. Set desc.[[Get]] to getter.
                desc.get = Some(Some(getter.unbind()));
            } else {
                desc.get = Some(None);
            }
        }
        // 13. Let hasSet be ? HasProperty(Obj, "set").
        let has_set = has_property(
            agent,
            scoped_obj.get(agent),
            BUILTIN_STRING_MEMORY.set.into(),
            gc.reborrow(),
        )
        .unbind()?;
        // 14. If hasSet is true, then
        if has_set {
            // a. Let setter be ? Get(Obj, "set").
            let setter = get(
                agent,
                scoped_obj.get(agent),
                BUILTIN_STRING_MEMORY.set.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. If IsCallable(setter) is false and setter is not undefined,
            // throw a TypeError exception.
            if !setter.is_undefined() {
                let Some(setter) = is_callable(setter, gc.nogc()) else {
                    return Err(agent.throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "setter is not callable",
                        gc.into_nogc(),
                    ));
                };
                // c. Set desc.[[Set]] to setter.
                desc.set = Some(Some(setter.unbind()));
            } else {
                desc.set = Some(None);
            }
        }

        // SAFETY: scoped_obj has not been shared.
        let _ = unsafe { scoped_obj.take(agent) };

        // 15. If desc has a [[Get]] field or desc has a [[Set]] field, then
        if desc.get.is_some() || desc.set.is_some() {
            // a. If desc has a [[Value]] field or desc has a [[Writable]]
            // field, throw a TypeError exception.
            if desc.value.is_some() || desc.writable.is_some() {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Over-defined property descriptor",
                    gc.into_nogc(),
                ));
            }
        }
        // 16. Return desc.
        Ok(desc)
    }

    pub(crate) fn try_to_property_descriptor(
        agent: &mut Agent,
        obj: Value,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<'a, Self> {
        // 1. If Obj is not an Object, throw a TypeError exception.
        let Ok(obj) = Object::try_from(obj) else {
            return agent
                .throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Property descriptor must be an object",
                    gc,
                )
                .into();
        };
        // 2. Let desc be a new Property Descriptor that initially has no
        // fields.
        let mut desc = PropertyDescriptor::default();
        // 3. Let hasEnumerable be ? HasProperty(Obj, "enumerable").
        let key = BUILTIN_STRING_MEMORY.enumerable.into();
        let cache = PropertyLookupCache::get(agent, key);
        let has_enumerable = match try_has_property(agent, obj, key, cache, gc) {
            ControlFlow::Continue(c) => match c {
                TryHasResult::Unset => false,
                TryHasResult::Offset(_, _) | TryHasResult::Custom(_, _) => true,
                TryHasResult::Proxy(_) => return TryError::GcError.into(),
            },
            ControlFlow::Break(_) => return TryError::GcError.into(),
        };
        // 4. If hasEnumerable is true, then
        if has_enumerable {
            // a. Let enumerable be ToBoolean(? Get(Obj, "enumerable")).
            let enumerable = try_get_result_into_value(try_get(agent, obj, key, cache, gc))?;
            let enumerable = to_boolean(agent, enumerable);
            // b. Set desc.[[Enumerable]] to enumerable.
            desc.enumerable = Some(enumerable);
        }
        // 5. Let hasConfigurable be ? HasProperty(Obj, "configurable").
        let key = BUILTIN_STRING_MEMORY.configurable.into();
        let cache = PropertyLookupCache::get(agent, key);
        let has_configurable = match try_has_property(agent, obj, key, cache, gc) {
            ControlFlow::Continue(c) => match c {
                TryHasResult::Unset => false,
                TryHasResult::Offset(_, _) | TryHasResult::Custom(_, _) => true,
                TryHasResult::Proxy(_) => return TryError::GcError.into(),
            },
            ControlFlow::Break(_) => return TryError::GcError.into(),
        };
        // 6. If hasConfigurable is true, then
        if has_configurable {
            // a. Let configurable be ToBoolean(? Get(Obj, "configurable")).
            let configurable = try_get_result_into_value(try_get(agent, obj, key, cache, gc))?;
            let configurable = to_boolean(agent, configurable);
            // b. Set desc.[[Configurable]] to configurable.
            desc.configurable = Some(configurable);
        }
        // 7. Let hasValue be ? HasProperty(Obj, "value").
        let key = BUILTIN_STRING_MEMORY.value.into();
        let cache = PropertyLookupCache::get(agent, key);
        let has_value = match try_has_property(agent, obj, key, cache, gc) {
            ControlFlow::Continue(c) => match c {
                TryHasResult::Unset => false,
                TryHasResult::Offset(_, _) | TryHasResult::Custom(_, _) => true,
                TryHasResult::Proxy(_) => return TryError::GcError.into(),
            },
            ControlFlow::Break(_) => return TryError::GcError.into(),
        };
        // 8. If hasValue is true, then
        if has_value {
            // a. Let value be ? Get(Obj, "value").
            let value = try_get_result_into_value(try_get(agent, obj, key, cache, gc))?;
            // b. Set desc.[[Value]] to value.
            desc.value = Some(value.unbind());
        }
        // 9. Let hasWritable be ? HasProperty(Obj, "writable").
        let key = BUILTIN_STRING_MEMORY.writable.into();
        let cache = PropertyLookupCache::get(agent, key);
        let has_writable = match try_has_property(agent, obj, key, cache, gc) {
            ControlFlow::Continue(c) => match c {
                TryHasResult::Unset => false,
                TryHasResult::Offset(_, _) | TryHasResult::Custom(_, _) => true,
                TryHasResult::Proxy(_) => return TryError::GcError.into(),
            },
            ControlFlow::Break(_) => return TryError::GcError.into(),
        };
        // 10. If hasWritable is true, then
        if has_writable {
            // a. Let writable be ToBoolean(? Get(Obj, "writable")).
            let writable = try_get_result_into_value(try_get(agent, obj, key, cache, gc))?;
            let writable = to_boolean(agent, writable);
            // b. Set desc.[[Writable]] to writable.
            desc.writable = Some(writable);
        }
        // 11. Let hasGet be ? HasProperty(Obj, "get").
        let key = BUILTIN_STRING_MEMORY.get.into();
        let cache = PropertyLookupCache::get(agent, key);
        let has_get = match try_has_property(agent, obj, key, cache, gc) {
            ControlFlow::Continue(c) => match c {
                TryHasResult::Unset => false,
                TryHasResult::Offset(_, _) | TryHasResult::Custom(_, _) => true,
                TryHasResult::Proxy(_) => return TryError::GcError.into(),
            },
            ControlFlow::Break(_) => return TryError::GcError.into(),
        };
        // 12. If hasGet is true, then
        if has_get {
            // a. Let getter be ? Get(Obj, "get").
            let getter = try_get_result_into_value(try_get(agent, obj, key, cache, gc))?;
            // b. If IsCallable(getter) is false and getter is not undefined,
            // throw a TypeError exception.
            if !getter.is_undefined() {
                let Some(getter) = is_callable(getter, gc) else {
                    return agent
                        .throw_exception_with_static_message(
                            ExceptionType::TypeError,
                            "getter is not callable",
                            gc,
                        )
                        .into();
                };
                // c. Set desc.[[Get]] to getter.
                desc.get = Some(Some(getter.unbind()));
            } else {
                desc.get = Some(None);
            }
        }
        // 13. Let hasSet be ? HasProperty(Obj, "set").
        let key = BUILTIN_STRING_MEMORY.set.into();
        let cache = PropertyLookupCache::get(agent, key);
        let has_set = match try_has_property(agent, obj, key, cache, gc) {
            ControlFlow::Continue(c) => match c {
                TryHasResult::Unset => false,
                TryHasResult::Offset(_, _) | TryHasResult::Custom(_, _) => true,
                TryHasResult::Proxy(_) => return TryError::GcError.into(),
            },
            ControlFlow::Break(_) => return TryError::GcError.into(),
        };
        // 14. If hasSet is true, then
        if has_set {
            // a. Let setter be ? Get(Obj, "set").
            let setter = try_get_result_into_value(try_get(agent, obj, key, cache, gc))?;
            // b. If IsCallable(setter) is false and setter is not undefined,
            // throw a TypeError exception.
            if !setter.is_undefined() {
                let Some(setter) = is_callable(setter, gc) else {
                    return agent
                        .throw_exception_with_static_message(
                            ExceptionType::TypeError,
                            "setter is not callable",
                            gc,
                        )
                        .into();
                };
                // c. Set desc.[[Set]] to setter.
                desc.set = Some(Some(setter.unbind()));
            } else {
                desc.set = Some(None);
            }
        }
        // 15. If desc has a [[Get]] field or desc has a [[Set]] field, then
        if desc.get.is_some() || desc.set.is_some() {
            // a. If desc has a [[Value]] field or desc has a [[Writable]]
            // field, throw a TypeError exception.
            if desc.value.is_some() || desc.writable.is_some() {
                return agent
                    .throw_exception_with_static_message(
                        ExceptionType::TypeError,
                        "Over-defined property descriptor",
                        gc,
                    )
                    .into();
            }
        }
        // 16. Return desc.
        TryResult::Continue(desc)
    }

    /// ### [6.2.6.6 CompletePropertyDescriptor ( Desc )](https://tc39.es/ecma262/#sec-completepropertydescriptor)
    ///
    /// The abstract operation CompletePropertyDescriptor takes
    /// argument Desc (a Property Descriptor) and returns unused.
    pub(crate) fn complete_property_descriptor(&mut self) -> JsResult<'a, ()> {
        // 1. Let like be the Record { [[Value]]: undefined, [[Writable]]: false, [[Get]]: undefined, [[Set]]: undefined, [[Enumerable]]: false, [[Configurable]]: false }.
        let like = PropertyDescriptor {
            value: Some(Value::Undefined),
            writable: Some(false),
            get: Some(None),
            set: Some(None),
            enumerable: Some(false),
            configurable: Some(false),
        };
        // 2. If IsGenericDescriptor(Desc) is true or IsDataDescriptor(Desc) is true, then
        if self.is_generic_descriptor() || self.is_data_descriptor() {
            // a. If Desc does not have a [[Value]] field, set Desc.[[Value]] to like.[[Value]].
            if self.value.is_none() {
                self.value = like.value;
            };
            // b. If Desc does not have a [[Writable]] field, set Desc.[[Writable]] to like.[[Writable]].
            if self.writable.is_none() {
                self.writable = like.writable;
            };
        } else {
            // 3. Else,
            // a. If Desc does not have a [[Get]] field, set Desc.[[Get]] to like.[[Get]].
            if self.get.is_none() {
                self.get = like.get;
            };
            // b. If Desc does not have a [[Set]] field, set Desc.[[Set]] to like.[[Set]].
            if self.set.is_none() {
                self.set = like.set;
            };
        };
        // 4. If Desc does not have an [[Enumerable]] field, set Desc.[[Enumerable]] to like.[[Enumerable]].
        if self.enumerable.is_none() {
            self.enumerable = like.enumerable;
        };
        // 5. If Desc does not have a [[Configurable]] field, set Desc.[[Configurable]] to like.[[Configurable]].
        if self.configurable.is_none() {
            self.configurable = like.configurable;
        };
        // 6. Return unused.
        Ok(())
    }

    pub fn is_fully_populated(&self) -> bool {
        ((self.value.is_some() && self.writable.is_some())
            // A property descriptor can contain just get or set.
            || self.get.is_some() || self.set.is_some())
            && self.enumerable.is_some()
            && self.configurable.is_some()
    }

    pub fn has_fields(&self) -> bool {
        self.value.is_some()
            || self.writable.is_some()
            || self.get.is_some()
            || self.set.is_some()
            || self.enumerable.is_some()
            || self.configurable.is_some()
    }
}

bindable_handle!(PropertyDescriptor);
