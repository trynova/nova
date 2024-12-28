// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::engine::context::{GcScope, NoGcScope};
use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::same_value,
        builtins::ordinary::ordinary_get_own_property,
        execution::{agent::ExceptionType, Agent, JsResult},
        scripts_and_modules::module::ModuleIdentifier,
        types::{
            InternalMethods, InternalSlots, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyDescriptor, PropertyKey, String, Value,
        },
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use self::data::ModuleHeapData;

use super::ordinary::{
    ordinary_delete, ordinary_get, ordinary_own_property_keys, ordinary_try_define_own_property,
    ordinary_try_get, ordinary_try_has_property,
};

pub mod data;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Module(pub(crate) ModuleIdentifier);

impl From<Module> for ModuleIdentifier {
    fn from(val: Module) -> Self {
        val.0
    }
}

impl From<ModuleIdentifier> for Module {
    fn from(value: ModuleIdentifier) -> Self {
        Self(value)
    }
}

impl IntoValue for Module {
    fn into_value(self) -> Value {
        self.into()
    }
}

impl IntoObject for Module {
    fn into_object(self) -> Object {
        self.into()
    }
}

impl From<Module> for Value {
    fn from(val: Module) -> Self {
        Value::Module(val)
    }
}

impl From<Module> for Object {
    fn from(val: Module) -> Self {
        Object::Module(val)
    }
}

impl Index<Module> for Agent {
    type Output = ModuleHeapData;

    fn index(&self, index: Module) -> &Self::Output {
        &self.heap.modules[index]
    }
}

impl IndexMut<Module> for Agent {
    fn index_mut(&mut self, index: Module) -> &mut Self::Output {
        &mut self.heap.modules[index]
    }
}

impl Index<Module> for Vec<Option<ModuleHeapData>> {
    type Output = ModuleHeapData;

    fn index(&self, index: Module) -> &Self::Output {
        self.get(index.get_index())
            .expect("Module out of bounds")
            .as_ref()
            .expect("Module slot empty")
    }
}

impl IndexMut<Module> for Vec<Option<ModuleHeapData>> {
    fn index_mut(&mut self, index: Module) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Module out of bounds")
            .as_mut()
            .expect("Module slot empty")
    }
}

impl Module {
    pub(crate) const fn _def() -> Self {
        Self(ModuleIdentifier::from_u32(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }
}

impl InternalSlots for Module {
    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        assert!(agent[self].object_index.replace(backing_object).is_none());
    }

    fn create_backing_object(self, _: &mut Agent) -> OrdinaryObject {
        unreachable!();
    }

    fn internal_extensible(self, _agent: &Agent) -> bool {
        false
    }

    fn internal_set_extensible(self, _agent: &mut Agent, _value: bool) {}

    fn internal_prototype(self, _agent: &Agent) -> Option<Object> {
        None
    }

    fn internal_set_prototype(self, _agent: &mut Agent, _prototype: Option<Object>) {}
}

impl InternalMethods for Module {
    /// ### [10.4.6.1 \[\[GetPrototypeOf\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-getprototypeof)
    fn try_get_prototype_of(self, _: &mut Agent, _: NoGcScope<'_, '_>) -> Option<Option<Object>> {
        Some(None)
    }

    /// ### [10.4.6.2 \[\[SetPrototypeOf\]\] ( V )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-setprototypeof-v)
    fn try_set_prototype_of(
        self,
        _: &mut Agent,
        prototype: Option<Object>,
        _: NoGcScope<'_, '_>,
    ) -> Option<bool> {
        // This is what it all comes down to in the end.
        Some(prototype.is_none())
    }

    /// ### [10.4.6.3 \[\[IsExtensible\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-isextensible)
    fn try_is_extensible(self, _: &mut Agent, _: NoGcScope<'_, '_>) -> Option<bool> {
        Some(false)
    }

    /// ### [10.4.6.4 \[\[PreventExtensions\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-preventextensions)
    fn try_prevent_extensions(self, _: &mut Agent, _: NoGcScope<'_, '_>) -> Option<bool> {
        Some(true)
    }

    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> Option<Option<PropertyDescriptor>> {
        match property_key {
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return OrdinaryGetOwnProperty(O, P).
                Some(
                    self.get_backing_object(agent)
                        .and_then(|object| ordinary_get_own_property(agent, object, property_key)),
                )
            }
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                let key = match property_key {
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::Integer(_) | PropertyKey::Symbol(_) => unreachable!(),
                };
                let exports_contains_p = exports.contains(&key);
                // 3. If exports does not contain P, return undefined.
                if !exports_contains_p {
                    Some(None)
                } else {
                    // 4. Let value be ? O.[[Get]](P, O).
                    let value = self.try_get(agent, property_key, self.into_value(), gc)?;
                    // 5. Return PropertyDescriptor { [[Value]]: value, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: false }.
                    Some(Some(PropertyDescriptor {
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
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Option<PropertyDescriptor>> {
        if let Some(result) = self.try_get_own_property(agent, property_key, gc.nogc()) {
            return Ok(result);
        }
        match property_key {
            PropertyKey::Symbol(_) => {
                // This would've returned Some from try branch.
                unreachable!();
            }
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                let key = match property_key {
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::Integer(_) => todo!(),
                    PropertyKey::Symbol(_) => unreachable!(),
                };
                let exports_contains_p = exports.contains(&key);
                // 3. If exports does not contain P, return undefined.
                if !exports_contains_p {
                    Ok(None)
                } else {
                    // 4. Let value be ? O.[[Get]](P, O).
                    let value = self.internal_get(agent, property_key, self.into_value(), gc)?;
                    // 5. Return PropertyDescriptor { [[Value]]: value, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: false }.
                    Ok(Some(PropertyDescriptor {
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

    /// ### [10.4.6.6 \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-defineownproperty-p-desc)
    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope<'_, '_>,
    ) -> Option<bool> {
        match property_key {
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return ! OrdinaryDefineOwnProperty(O, P, Desc).
                Some(self.get_backing_object(agent).map_or(false, |object| {
                    ordinary_try_define_own_property(
                        agent,
                        object,
                        property_key,
                        property_descriptor,
                        gc,
                    )
                }))
            }
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let current be ? O.[[GetOwnProperty]](P).
                let current = self.try_get_own_property(agent, property_key, gc)?;
                // 3. If current is undefined, return false.
                let Some(current) = current else {
                    return Some(false);
                };
                // 4. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
                if property_descriptor.configurable == Some(true) {
                    return Some(false);
                }
                // 5. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]] is false, return false.
                if property_descriptor.enumerable == Some(false) {
                    return Some(false);
                }
                // 6. If IsAccessorDescriptor(Desc) is true, return false.
                if property_descriptor.is_accessor_descriptor() {
                    return Some(false);
                }
                // 7. If Desc has a [[Writable]] field and Desc.[[Writable]] is false, return false.
                if property_descriptor.writable == Some(false) {
                    return Some(false);
                }
                // 8. If Desc has a [[Value]] field, return SameValue(Desc.[[Value]], current.[[Value]]).
                if let Some(value) = property_descriptor.value {
                    Some(same_value(agent, value, current.value.unwrap()))
                } else {
                    // 9. Return true.
                    Some(true)
                }
            }
        }
    }

    /// ### [10.4.6.6 \[\[DefineOwnProperty\]\] ( P, Desc )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-defineownproperty-p-desc)
    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        match property_key {
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return ! OrdinaryDefineOwnProperty(O, P, Desc).
                Ok(self.get_backing_object(agent).map_or(false, |object| {
                    ordinary_try_define_own_property(
                        agent,
                        object,
                        property_key,
                        property_descriptor,
                        gc.into_nogc(),
                    )
                }))
            }
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let current be ? O.[[GetOwnProperty]](P).
                let current = self.internal_get_own_property(agent, property_key, gc)?;
                // 3. If current is undefined, return false.
                let Some(current) = current else {
                    return Ok(false);
                };
                // 4. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
                if property_descriptor.configurable == Some(true) {
                    return Ok(false);
                }
                // 5. If Desc has an [[Enumerable]] field and Desc.[[Enumerable]] is false, return false.
                if property_descriptor.enumerable == Some(false) {
                    return Ok(false);
                }
                // 6. If IsAccessorDescriptor(Desc) is true, return false.
                if property_descriptor.is_accessor_descriptor() {
                    return Ok(false);
                }
                // 7. If Desc has a [[Writable]] field and Desc.[[Writable]] is false, return false.
                if property_descriptor.writable == Some(false) {
                    return Ok(false);
                }
                // 8. If Desc has a [[Value]] field, return SameValue(Desc.[[Value]], current.[[Value]]).
                if let Some(value) = property_descriptor.value {
                    Ok(same_value(agent, value, current.value.unwrap()))
                } else {
                    // 9. Return true.
                    Ok(true)
                }
            }
        }
    }

    /// ### [10.4.6.7 \[\[HasProperty\]\] ( P )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-hasproperty-p)
    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> Option<bool> {
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
                    Some(true)
                } else {
                    // 4. Return false.
                    Some(false)
                }
            }
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return ! OrdinaryHasProperty(O, P).
                Some(self.get_backing_object(agent).map_or(false, |object| {
                    ordinary_try_has_property(agent, object, property_key, gc).unwrap()
                }))
            }
        }
    }

    /// ### [10.4.6.8 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-get-p-receiver)
    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> Option<Value> {
        // NOTE: ResolveExport is side-effect free. Each time this operation
        // is called with a specific exportName, resolveSet pair as arguments
        // it must return the same result. An implementation might choose to
        // pre-compute or cache the ResolveExport results for the [[Exports]]
        // of each module namespace exotic object.

        match property_key {
            // 1. If P is a Symbol, then
            PropertyKey::Symbol(_) => {
                // a. Return ! OrdinaryGet(O, P, Receiver).
                Some(
                    self.get_backing_object(agent)
                        .map_or(Value::Undefined, |object| {
                            ordinary_try_get(agent, object, property_key, receiver, gc).unwrap()
                        }),
                )
            }
            PropertyKey::Integer(_) | PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                let key = match property_key {
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::Integer(_) => todo!(),
                    PropertyKey::Symbol(_) => unreachable!(),
                };
                let exports_contains_p = exports.contains(&key);
                // 3. If exports does not contain P, return undefined.
                if !exports_contains_p {
                    Some(Value::Undefined)
                } else {
                    // 4. Let m be O.[[Module]].
                    let m = &agent[self].module;
                    // 5. Let binding be m.ResolveExport(P).
                    let binding = m.resolve_export(property_key);
                    // 6. Assert: binding is a ResolvedBinding Record.
                    let Some(data::ResolveExportResult::Resolved(binding)) = binding else {
                        unreachable!();
                    };
                    // 7. Let targetModule be binding.[[Module]].
                    // 8. Assert: targetModule is not undefined.
                    let target_module = binding.module.unwrap();
                    // 9. If binding.[[BindingName]] is NAMESPACE, then
                    let _binding_name = match binding.binding_name {
                        data::ResolvedBindingName::Namespace => {
                            // a. Return GetModuleNamespace(targetModule).
                            todo!();
                        }
                        data::ResolvedBindingName::String(data) => String::String(data),
                        data::ResolvedBindingName::SmallString(data) => String::SmallString(data),
                    };
                    // 10. Let targetEnv be targetModule.[[Environment]].
                    let target_env = agent[target_module].module.environment;
                    // 11. If targetEnv is EMPTY, throw a ReferenceError exception.
                    match target_env {
                        None => None,
                        Some(_target_env) => {
                            // 12. Return ? targetEnv.GetBindingValue(binding.[[BindingName]], true).
                            todo!()
                        }
                    }
                }
            }
        }
    }

    /// ### [10.4.6.8 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-get-p-receiver)
    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        // NOTE: ResolveExport is side-effect free. Each time this operation
        // is called with a specific exportName, resolveSet pair as arguments
        // it must return the same result. An implementation might choose to
        // pre-compute or cache the ResolveExport results for the [[Exports]]
        // of each module namespace exotic object.

        match property_key {
            // 1. If P is a Symbol, then
            PropertyKey::Symbol(_) => {
                // a. Return ! OrdinaryGet(O, P, Receiver).
                Ok(self
                    .get_backing_object(agent)
                    .map_or(Value::Undefined, |object| {
                        ordinary_get(agent, object, property_key, receiver, gc.reborrow()).unwrap()
                    }))
            }
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
                    Ok(Value::Undefined)
                } else {
                    // 4. Let m be O.[[Module]].
                    let m = &agent[self].module;
                    // 5. Let binding be m.ResolveExport(P).
                    let binding = m.resolve_export(property_key);
                    // 6. Assert: binding is a ResolvedBinding Record.
                    let Some(data::ResolveExportResult::Resolved(binding)) = binding else {
                        unreachable!();
                    };
                    // 7. Let targetModule be binding.[[Module]].
                    // 8. Assert: targetModule is not undefined.
                    let target_module = binding.module.unwrap();
                    // 9. If binding.[[BindingName]] is NAMESPACE, then
                    let _binding_name = match binding.binding_name {
                        data::ResolvedBindingName::Namespace => {
                            // a. Return GetModuleNamespace(targetModule).
                            todo!();
                        }
                        data::ResolvedBindingName::String(data) => String::String(data),
                        data::ResolvedBindingName::SmallString(data) => String::SmallString(data),
                    };
                    // 10. Let targetEnv be targetModule.[[Environment]].
                    let target_env = agent[target_module].module.environment;
                    // 11. If targetEnv is EMPTY, throw a ReferenceError exception.
                    match target_env {
                        None => Err(agent.throw_exception(
                            ExceptionType::ReferenceError,
                            format!("Could not resolve module '{}'.", key.as_str(agent)),
                            gc.nogc(),
                        )),
                        Some(_target_env) => {
                            // 12. Return ? targetEnv.GetBindingValue(binding.[[BindingName]], true).
                            todo!()
                        }
                    }
                }
            }
        }
    }

    /// ### [10.4.6.9 \[\[Set\]\] ( P, V, Receiver )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-set-p-v-receiver)
    fn try_set(
        self,
        _: &mut Agent,
        _: PropertyKey,
        _: Value,
        _: Value,
        _: NoGcScope<'_, '_>,
    ) -> Option<bool> {
        Some(false)
    }

    /// ### [10.4.6.10 \[\[Delete\]\] ( P )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-delete-p)
    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> Option<bool> {
        match property_key {
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, then
                // a. Return ! OrdinaryDelete(O, P).
                Some(self.get_backing_object(agent).map_or(true, |object| {
                    ordinary_delete(agent, object, property_key, gc)
                }))
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
                // 3. If exports contains P, return false.
                if exports.contains(&p) {
                    Some(false)
                } else {
                    // 4. Return true.
                    Some(true)
                }
            }
        }
    }

    /// ### [10.4.6.11 \[\[OwnPropertyKeys\]\] ( )])(https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-ownpropertykeys)
    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> Option<Vec<PropertyKey<'gc>>> {
        // 1. Let exports be O.[[Exports]].
        let exports = agent[self]
            .exports
            .iter()
            .map(|string| PropertyKey::from(*string));
        let exports_count = exports.len();
        // 2. Let symbolKeys be OrdinaryOwnPropertyKeys(O).
        let symbol_keys = self.get_backing_object(agent).map_or(vec![], |object| {
            ordinary_own_property_keys(agent, object, gc)
        });
        let symbol_keys_count = symbol_keys.len();
        // 3. Return the list-concatenation of exports and symbolKeys.
        let mut own_property_keys = Vec::with_capacity(exports_count + symbol_keys_count);
        exports.for_each(|export_key| own_property_keys.push(export_key));
        symbol_keys
            .iter()
            .for_each(|symbol_key| own_property_keys.push(*symbol_key));
        Some(own_property_keys)
    }
}

impl HeapMarkAndSweep for Module {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.modules.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.0.into_u32();
        self.0 = ModuleIdentifier::from_u32(
            self_index - compactions.modules.get_shift_for_index(self_index),
        );
    }
}
