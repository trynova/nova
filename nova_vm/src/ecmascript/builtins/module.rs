// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};
use std::{marker::PhantomData, ops::ControlFlow};

use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::same_value,
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, TryError, TryResult},
            throw_uninitialized_binding,
        },
        scripts_and_modules::module::module_semantics::{
            abstract_module_records::{
                AbstractModule, AbstractModuleMethods, AbstractModuleSlots, ResolvedBinding,
            },
            get_module_namespace,
        },
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, IntoObject, IntoValue, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, SetResult, String, TryGetResult,
            TryHasResult, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, Scopable},
    },
    heap::{
        CompactionLists, CreateHeapData, HeapMarkAndSweep, HeapSweepWeakReference,
        WellKnownSymbolIndexes, WorkQueues,
    },
};

use self::data::ModuleHeapData;

use super::ordinary::{
    caches::{PropertyLookupCache, PropertyOffset},
    shape::ObjectShape,
};

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Module<'a>(u32, PhantomData<&'a ()>);

impl<'a> From<Module<'a>> for Value<'a> {
    fn from(value: Module<'a>) -> Self {
        Value::Module(value)
    }
}

impl<'a> From<Module<'a>> for Object<'a> {
    fn from(value: Module<'a>) -> Self {
        Object::Module(value)
    }
}

impl Index<Module<'_>> for Agent {
    type Output = ModuleHeapData<'static>;

    fn index(&self, index: Module) -> &Self::Output {
        &self.heap.modules[index]
    }
}

impl IndexMut<Module<'_>> for Agent {
    fn index_mut(&mut self, index: Module) -> &mut Self::Output {
        &mut self.heap.modules[index]
    }
}

impl Index<Module<'_>> for Vec<ModuleHeapData<'static>> {
    type Output = ModuleHeapData<'static>;

    fn index(&self, index: Module) -> &Self::Output {
        self.get(index.get_index()).expect("Module out of bounds")
    }
}

impl IndexMut<Module<'_>> for Vec<ModuleHeapData<'static>> {
    fn index_mut(&mut self, index: Module) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Module out of bounds")
    }
}

impl Module<'_> {
    pub(crate) const fn _def() -> Self {
        Self::from_u32(0)
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0 as usize
    }

    /// Creates a module identififer from a usize.
    ///
    /// ## Panics
    /// If the given index is greater than `u32::MAX`.
    pub(crate) const fn from_index(value: usize) -> Self {
        assert!(value <= u32::MAX as usize);
        Self(value as u32, PhantomData)
    }

    /// Creates a module identififer from a u32.
    pub(crate) const fn from_u32(value: u32) -> Self {
        Self(value, PhantomData)
    }
}

bindable_handle!(Module);

impl<'a> InternalSlots<'a> for Module<'a> {
    #[inline(always)]
    fn get_backing_object(self, _agent: &Agent) -> Option<OrdinaryObject<'static>> {
        None
    }

    #[inline(always)]
    fn set_backing_object(self, _agent: &mut Agent, _backing_object: OrdinaryObject<'static>) {
        unreachable!()
    }

    #[inline(always)]
    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject<'static> {
        unreachable!();
    }

    #[inline(always)]
    fn internal_extensible(self, _agent: &Agent) -> bool {
        unreachable!()
    }

    #[inline(always)]
    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {
        unreachable!()
    }

    #[inline(always)]
    fn internal_prototype(self, _agent: &Agent) -> Option<Object<'static>> {
        None
    }

    #[inline(always)]
    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {
        unreachable!()
    }

    #[inline(always)]
    fn object_shape(self, _: &mut Agent) -> ObjectShape<'static> {
        unreachable!()
    }
}

impl<'a> InternalMethods<'a> for Module<'a> {
    /// ### [10.4.6.1 \[\[GetPrototypeOf\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-getprototypeof)
    fn try_get_prototype_of<'gc>(
        self,
        _: &mut Agent,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<Object<'gc>>> {
        TryResult::Continue(None)
    }

    /// ### [10.4.6.2 \[\[SetPrototypeOf\]\] ( V )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-setprototypeof-v)
    fn try_set_prototype_of<'gc>(
        self,
        _: &mut Agent,
        prototype: Option<Object>,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        // This is what it all comes down to in the end.
        TryResult::Continue(prototype.is_none())
    }

    /// ### [10.4.6.3 \[\[IsExtensible\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-isextensible)
    fn try_is_extensible<'gc>(self, _: &mut Agent, _: NoGcScope<'gc, '_>) -> TryResult<'gc, bool> {
        TryResult::Continue(false)
    }

    /// ### [10.4.6.4 \[\[PreventExtensions\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-preventextensions)
    fn try_prevent_extensions<'gc>(
        self,
        _: &mut Agent,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        TryResult::Continue(true)
    }

    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        match property_key {
            PropertyKey::Symbol(symbol) => {
                // 1. If P is a Symbol, return OrdinaryGetOwnProperty(O, P).
                if symbol == WellKnownSymbolIndexes::ToStringTag.into() {
                    TryResult::Continue(Some(PropertyDescriptor {
                        value: Some(BUILTIN_STRING_MEMORY.Module.into_value()),
                        writable: Some(false),
                        get: None,
                        set: None,
                        enumerable: Some(false),
                        configurable: Some(false),
                    }))
                } else {
                    TryResult::Continue(None)
                }
            }
            PropertyKey::PrivateName(_) => unreachable!(),
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                let key = match property_key {
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::Integer(data) => {
                        String::from_string(agent, format!("{}", data.into_i64()), gc)
                    }
                    PropertyKey::Symbol(_) | PropertyKey::PrivateName(_) => unreachable!(),
                };
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                let exports_contains_p = exports.contains(&key);
                // 3. If exports does not contain P, return undefined.
                if !exports_contains_p {
                    TryResult::Continue(None)
                } else {
                    // 4. Let value be ? O.[[Get]](P, O).
                    let value = match self.try_get(agent, property_key, self.into_value(), None, gc)
                    {
                        ControlFlow::Continue(TryGetResult::Unset) => Value::Undefined,
                        ControlFlow::Continue(TryGetResult::Value(v)) => v,
                        _ => return TryError::GcError.into(),
                    };
                    // 5. Return PropertyDescriptor { [[Value]]: value, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: false }.
                    TryResult::Continue(Some(PropertyDescriptor {
                        value: Some(value),
                        writable: Some(true),
                        get: None,
                        set: None,
                        enumerable: Some(true),
                        configurable: Some(false),
                    }))
                }
            }
        }
    }

    /// 10.4.6.5 \[\[GetOwnProperty\]\] ( P )
    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
        let property_key = property_key.bind(gc.nogc());
        match property_key {
            PropertyKey::Symbol(symbol) => {
                // 1. If P is a Symbol, return OrdinaryGetOwnProperty(O, P).
                if symbol == WellKnownSymbolIndexes::ToStringTag.into() {
                    Ok(Some(PropertyDescriptor {
                        value: Some(BUILTIN_STRING_MEMORY.Module.into_value()),
                        writable: Some(false),
                        get: None,
                        set: None,
                        enumerable: Some(false),
                        configurable: Some(false),
                    }))
                } else {
                    Ok(None)
                }
            }
            PropertyKey::PrivateName(_) => unreachable!(),
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                let key = match property_key {
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::Integer(data) => {
                        String::from_string(agent, format!("{}", data.into_i64()), gc.nogc())
                    }
                    PropertyKey::Symbol(_) | PropertyKey::PrivateName(_) => unreachable!(),
                };
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                let exports_contains_p = exports.contains(&key);
                // 3. If exports does not contain P, return undefined.
                if !exports_contains_p {
                    Ok(None)
                } else {
                    // 4. Let value be ? O.[[Get]](P, O).
                    let value =
                        self.internal_get(agent, property_key.unbind(), self.into_value(), gc)?;
                    // 5. Return PropertyDescriptor { [[Value]]: value, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: false }.
                    Ok(Some(PropertyDescriptor {
                        value: Some(value.unbind()),
                        writable: Some(true),
                        get: None,
                        set: None,
                        enumerable: Some(true),
                        configurable: Some(false),
                    }))
                }
            }
        }
    }

    /// ### [10.4.6.6 \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-defineownproperty-p-desc)
    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match property_key {
            PropertyKey::Symbol(symbol) => {
                // 1. If P is a Symbol, return ! OrdinaryDefineOwnProperty(O, P, Desc).
                if symbol == WellKnownSymbolIndexes::ToStringTag.into() {
                    // Note: it's always okay for a field to not exist on the
                    // descriptor. It just means that the defineOwnProperty
                    // isn't trying to change it. Hence the map_or checks below.
                    TryResult::Continue(
                        property_descriptor
                            .value
                            .is_none_or(|v| v == BUILTIN_STRING_MEMORY.Module.into_value())
                            && property_descriptor.writable.is_none_or(|v| !v)
                            && property_descriptor.get.is_none()
                            && property_descriptor.set.is_none()
                            && property_descriptor.enumerable.is_none_or(|v| !v)
                            && property_descriptor.configurable.is_none_or(|v| !v),
                    )
                } else {
                    TryResult::Continue(false)
                }
            }
            PropertyKey::PrivateName(_) => unreachable!(),
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let current be ? O.[[GetOwnProperty]](P).
                let current = self.try_get_own_property(agent, property_key, cache, gc)?;
                // 3. If current is undefined, return false.
                let Some(current) = current else {
                    return TryResult::Continue(false);
                };
                // 4. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
                if property_descriptor.configurable == Some(true) {
                    return TryResult::Continue(false);
                }
                // 5. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]] is false, return false.
                if property_descriptor.enumerable == Some(false) {
                    return TryResult::Continue(false);
                }
                // 6. If IsAccessorDescriptor(Desc) is true, return false.
                if property_descriptor.is_accessor_descriptor() {
                    return TryResult::Continue(false);
                }
                // 7. If Desc has a [[Writable]] field and Desc.[[Writable]] is false, return false.
                if property_descriptor.writable == Some(false) {
                    return TryResult::Continue(false);
                }
                // 8. If Desc has a [[Value]] field, return SameValue(Desc.[[Value]], current.[[Value]]).
                if let Some(value) = property_descriptor.value {
                    TryResult::Continue(same_value(agent, value, current.value.unwrap()))
                } else {
                    // 9. Return true.
                    TryResult::Continue(true)
                }
            }
        }
    }

    /// ### [10.4.6.6 \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-defineownproperty-p-desc)
    fn internal_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let o = self.bind(gc.nogc());
        let property_key = property_key.bind(gc.nogc());
        let property_descriptor = property_descriptor.bind(gc.nogc());
        match property_key {
            PropertyKey::Symbol(symbol) => {
                // 1. If P is a Symbol, return ! OrdinaryDefineOwnProperty(O, P, Desc).
                if symbol == WellKnownSymbolIndexes::ToStringTag.into() {
                    // Note: it's always okay for a field to not exist on the
                    // descriptor. It just means that the defineOwnProperty
                    // isn't trying to change it. Hence the is_none_or usage
                    // below.
                    Ok(property_descriptor
                        .value
                        .is_none_or(|v| v == BUILTIN_STRING_MEMORY.Module.into_value())
                        && property_descriptor.writable.is_none_or(|v| !v)
                        && property_descriptor.get.is_none()
                        && property_descriptor.set.is_none()
                        && property_descriptor.enumerable.is_none_or(|v| !v)
                        && property_descriptor.configurable.is_none_or(|v| !v))
                } else {
                    Ok(false)
                }
            }
            PropertyKey::PrivateName(_) => unreachable!(),
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let current be ? O.[[GetOwnProperty]](P).
                let is_accessor_descriptor = property_descriptor.is_accessor_descriptor();
                let PropertyDescriptor {
                    value,
                    writable,
                    enumerable,
                    configurable,
                    ..
                } = property_descriptor;
                let value = value.map(|v| v.scope(agent, gc.nogc()));
                let current =
                    o.unbind()
                        .internal_get_own_property(agent, property_key.unbind(), gc)?;
                // 3. If current is undefined, return false.
                let Some(current) = current else {
                    return Ok(false);
                };
                // 4. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
                if configurable == Some(true) {
                    return Ok(false);
                }
                // 5. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]] is false, return false.
                if enumerable == Some(false) {
                    return Ok(false);
                }
                // 6. If IsAccessorDescriptor(Desc) is true, return false.
                if is_accessor_descriptor {
                    return Ok(false);
                }
                // 7. If Desc has a [[Writable]] field and Desc.[[Writable]] is false, return false.
                if writable == Some(false) {
                    return Ok(false);
                }
                // 8. If Desc has a [[Value]] field, return SameValue(Desc.[[Value]], current.[[Value]]).
                if let Some(value) = value {
                    Ok(same_value(
                        agent,
                        value.get(agent),
                        current.value.unwrap_or(Value::Undefined),
                    ))
                } else {
                    // 9. Return true.
                    Ok(true)
                }
            }
        }
    }

    /// ### [10.4.6.7 \[\[HasProperty\]\] ( P )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-hasproperty-p)
    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        match property_key {
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                let p = match property_key {
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::Integer(_data) => todo!(),
                    _ => unreachable!(),
                };
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                // 3. If exports contains P, return true.
                if exports.contains(&p) {
                    TryHasResult::Custom(1, self.into_object().bind(gc)).into()
                } else {
                    // 4. Return false.
                    TryHasResult::Unset.into()
                }
            }
            PropertyKey::Symbol(symbol) => {
                // 1. If P is a Symbol, return ! OrdinaryHasProperty(O, P).
                if symbol == WellKnownSymbolIndexes::ToStringTag.into() {
                    TryHasResult::Custom(0, self.into_object().bind(gc)).into()
                } else {
                    TryHasResult::Unset.into()
                }
            }
            PropertyKey::PrivateName(_) => unreachable!(),
        }
    }

    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        Ok(!matches!(
            self.try_has_property(agent, property_key, None, gc.into_nogc()),
            ControlFlow::Continue(TryHasResult::Unset)
        ))
    }

    /// ### [10.4.6.8 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-get-p-receiver)
    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _receiver: Value,
        _cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        // NOTE: ResolveExport is side-effect free. Each time this operation
        // is called with a specific exportName, resolveSet pair as arguments
        // it must return the same result. An implementation might choose to
        // pre-compute or cache the ResolveExport results for the [[Exports]]
        // of each module namespace exotic object.

        match property_key {
            // 1. If P is a Symbol, then
            PropertyKey::Symbol(symbol) => {
                // a. Return ! OrdinaryGet(O, P, Receiver).
                if symbol == WellKnownSymbolIndexes::ToStringTag.into() {
                    TryGetResult::Value(BUILTIN_STRING_MEMORY.Module.into_value()).into()
                } else {
                    TryGetResult::Unset.into()
                }
            }
            PropertyKey::PrivateName(_) => unreachable!(),
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                let key = match property_key {
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::Integer(_) => todo!(),
                    _ => unreachable!(),
                };
                let exports_contains_p = exports.contains(&key);
                // 3. If exports does not contain P, return undefined.
                if !exports_contains_p {
                    TryGetResult::Unset.into()
                } else {
                    // 4. Let m be O.[[Module]].
                    let m = &agent[self].module;
                    // 5. Let binding be m.ResolveExport(P).
                    let binding = m.resolve_export(agent, key, &mut vec![], gc);
                    // 6. Assert: binding is a ResolvedBinding Record.
                    let Some(ResolvedBinding::Resolved {
                        // 7. Let targetModule be binding.[[Module]].
                        // 8. Assert: targetModule is not undefined.
                        module: target_module,
                        binding_name,
                    }) = binding
                    else {
                        unreachable!();
                    };
                    // 9. If binding.[[BindingName]] is NAMESPACE, then
                    let Some(binding_name) = binding_name else {
                        // a. Return GetModuleNamespace(targetModule).
                        return TryGetResult::Value(
                            get_module_namespace(agent, target_module.unbind(), gc).into_value(),
                        )
                        .into();
                    };
                    // 10. Let targetEnv be targetModule.[[Environment]].
                    let target_env = target_module.environment(agent, gc);
                    // 11. If targetEnv is EMPTY, throw a ReferenceError exception.
                    let Some(target_env) = target_env else {
                        return agent
                            .throw_exception_with_static_message(
                                ExceptionType::ReferenceError,
                                "Attempted to access unlinked module's environment",
                                gc,
                            )
                            .into();
                    };
                    // 12. Return ? targetEnv.GetBindingValue(binding.[[BindingName]], true).
                    if let Some(value) = target_env.get_binding_value(agent, binding_name, true, gc)
                    {
                        TryGetResult::Value(value).into()
                    } else {
                        throw_uninitialized_binding(agent, binding_name, gc).into()
                    }
                }
            }
        }
    }

    /// ### [10.4.6.8 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-get-p-receiver)
    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let gc = gc.into_nogc();
        let property_key = property_key.bind(gc);

        // NOTE: ResolveExport is side-effect free. Each time this operation
        // is called with a specific exportName, resolveSet pair as arguments
        // it must return the same result. An implementation might choose to
        // pre-compute or cache the ResolveExport results for the [[Exports]]
        // of each module namespace exotic object.

        match property_key {
            // 1. If P is a Symbol, then
            PropertyKey::Symbol(symbol) => {
                // a. Return ! OrdinaryGet(O, P, Receiver).
                if symbol == WellKnownSymbolIndexes::ToStringTag.into() {
                    Ok(BUILTIN_STRING_MEMORY.Module.into_value())
                } else {
                    Ok(Value::Undefined)
                }
            }
            PropertyKey::PrivateName(_) => unreachable!(),
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                let key = match property_key {
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::Integer(_) => todo!(),
                    _ => unreachable!(),
                };
                let exports_contains_p = exports.contains(&key);
                // 3. If exports does not contain P,
                if !exports_contains_p {
                    // return undefined.
                    Ok(Value::Undefined)
                } else {
                    // 4. Let m be O.[[Module]].
                    let m = &agent[self].module;
                    // 5. Let binding be m.ResolveExport(P).
                    let binding = m.resolve_export(agent, key, &mut vec![], gc);
                    // 6. Assert: binding is a ResolvedBinding Record.
                    let Some(ResolvedBinding::Resolved {
                        // 7. Let targetModule be binding.[[Module]].
                        // 8. Assert: targetModule is not undefined.
                        module: target_module,
                        binding_name,
                    }) = binding
                    else {
                        unreachable!();
                    };
                    // 9. If binding.[[BindingName]] is NAMESPACE, then
                    let Some(binding_name) = binding_name else {
                        // a. Return GetModuleNamespace(targetModule).
                        return Ok(
                            get_module_namespace(agent, target_module.unbind(), gc).into_value()
                        );
                    };
                    // 10. Let targetEnv be targetModule.[[Environment]].
                    let target_env = target_module.environment(agent, gc);
                    // 11. If targetEnv is EMPTY, throw a ReferenceError exception.
                    let Some(target_env) = target_env else {
                        return Err(agent.throw_exception(
                            ExceptionType::ReferenceError,
                            format!("Could not resolve module '{}'.", key.to_string_lossy(agent)),
                            gc,
                        ));
                    };
                    // 12. Return ? targetEnv.GetBindingValue(binding.[[BindingName]], true).
                    if let Some(value) = target_env.get_binding_value(agent, binding_name, true, gc)
                    {
                        Ok(value)
                    } else {
                        Err(throw_uninitialized_binding(agent, binding_name, gc))
                    }
                }
            }
        }
    }

    /// ### [10.4.6.9 \[\[Set\]\] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-set-p-v-receiver)
    fn try_set<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Value,
        _: Value,
        _: Option<PropertyLookupCache>,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        SetResult::Unwritable.into()
    }

    fn internal_set<'gc>(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Value,
        _: Value,
        _: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        Ok(false)
    }

    /// ### [10.4.6.10 \[\[Delete\]\] ( P )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-delete-p)
    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        match property_key {
            PropertyKey::Symbol(symbol) => {
                // 1. If P is a Symbol, then
                // a. Return ! OrdinaryDelete(O, P).
                TryResult::Continue(symbol != WellKnownSymbolIndexes::ToStringTag.into())
            }
            PropertyKey::PrivateName(_) => {
                unreachable!()
            }
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                let p = match property_key {
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::Integer(_) => todo!(),
                    _ => unreachable!(),
                };
                // 2. Let exports be O.[[Exports]].
                let exports = &agent[self].exports;
                // 3. If exports contains P,
                if exports.contains(&p) {
                    // return false.
                    TryResult::Continue(false)
                } else {
                    // 4. Return true.
                    TryResult::Continue(true)
                }
            }
        }
    }

    /// ### [10.4.6.11 \[\[OwnPropertyKeys\]\] ( )])(https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-ownpropertykeys)
    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        _gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
        // 1. Let exports be O.[[Exports]].
        let exports = agent[self]
            .exports
            .iter()
            .map(|string| PropertyKey::from(*string));
        let exports_count = exports.len();
        // 2. Let symbolKeys be OrdinaryOwnPropertyKeys(O).
        // 3. Return the list-concatenation of exports and symbolKeys.
        let mut own_property_keys = Vec::with_capacity(exports_count + 1);
        exports.for_each(|export_key| own_property_keys.push(export_key));
        own_property_keys.push(WellKnownSymbolIndexes::ToStringTag.into());
        TryResult::Continue(own_property_keys)
    }

    #[inline(always)]
    fn get_own_property_at_offset<'gc>(
        self,
        _: &Agent,
        _: PropertyOffset,
        _: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        unreachable!()
    }
}

/// ### [10.4.6.12 ModuleNamespaceCreate ( module, exports )](https://tc39.es/ecma262/#sec-modulenamespacecreate)
///
/// The abstract operation ModuleNamespaceCreate takes arguments module (a
/// Module Record) and exports (a List of Strings) and returns a module
/// namespace exotic object. It is used to specify the creation of new module
/// namespace exotic objects.
pub(crate) fn module_namespace_create<'a>(
    agent: &mut Agent,
    module: AbstractModule<'a>,
    mut exports: Box<[String<'a>]>,
    gc: NoGcScope<'a, '_>,
) -> Module<'a> {
    // 1. Assert: module.[[Namespace]] is empty.
    debug_assert!(module.namespace(agent, gc).is_none());
    // 2. Let internalSlotsList be the internal slots listed in Table 33.
    // 3. Let M be MakeBasicObject(internalSlotsList).
    // 4. Set M's essential internal methods to the definitions specified in 10.4.6.
    // 5. Set M.[[Module]] to module.
    // 6. Let sortedExports be a List whose elements are the elements of
    //    exports, sorted according to lexicographic code unit order.
    // TODO: this implements UTF-8 lexicographic order, not UTF-16.
    exports.sort_by(|a, b| a.as_wtf8(agent).cmp(b.as_wtf8(agent)));
    // 7. Set M.[[Exports]] to sortedExports.
    // 8. Create own properties of M corresponding to the definitions in 28.3.
    let m = agent.heap.create(ModuleHeapData { module, exports });
    // 9. Set module.[[Namespace]] to M.
    module.set_namespace(agent, m);
    // 10. Return M.
    m
}

impl TryFrom<HeapRootData> for Module<'_> {
    type Error = ();

    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        if let HeapRootData::Module(value) = value {
            Ok(value)
        } else {
            Err(())
        }
    }
}

impl HeapMarkAndSweep for Module<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.modules.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.modules.shift_u32_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Module<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .modules
            .shift_weak_u32_index(self.0)
            .map(Self::from_u32)
    }
}
