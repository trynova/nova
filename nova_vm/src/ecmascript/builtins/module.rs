use std::ops::{Index, IndexMut};

use crate::{
    ecmascript::{
        abstract_operations::testing_and_comparison::same_value,
        builtins::ordinary::ordinary_get_own_property,
        execution::{agent::ExceptionType, Agent, JsResult},
        scripts_and_modules::module::ModuleIdentifier,
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
            PropertyDescriptor, PropertyKey, String, Value,
        },
    },
    Heap,
};

use self::data::ModuleHeapData;

use super::ordinary::{
    ordinary_define_own_property, ordinary_delete, ordinary_get, ordinary_has_property,
    ordinary_own_property_keys, set_immutable_prototype,
};

pub(crate) mod abstract_module_records;
pub(crate) mod cyclic_module_records;
pub mod data;
pub(crate) mod semantics;
pub(crate) mod source_text_module_records;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        Value::Module(val.0)
    }
}

impl From<Module> for Object {
    fn from(val: Module) -> Self {
        Object::Module(val.0)
    }
}

impl Index<Module> for Agent {
    type Output = ModuleHeapData;

    fn index(&self, index: Module) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<Module> for Agent {
    fn index_mut(&mut self, index: Module) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<Module> for Heap {
    type Output = ModuleHeapData;

    fn index(&self, index: Module) -> &Self::Output {
        self.modules
            .get(index.0.into_index())
            .expect("Module out of bounds")
            .as_ref()
            .expect("Module slot empty")
    }
}

impl IndexMut<Module> for Heap {
    fn index_mut(&mut self, index: Module) -> &mut Self::Output {
        self.modules
            .get_mut(index.0.into_index())
            .expect("Module out of bounds")
            .as_mut()
            .expect("Module slot empty")
    }
}

impl Module {
    fn get_backing_object(self, agent: &Agent) -> Option<Object> {
        agent[self].object_index.map(|idx| idx.into())
    }
}

impl OrdinaryObjectInternalSlots for Module {
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
    fn internal_get_prototype_of(self, _agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(None)
    }

    /// ### [10.4.6.2 \[\[SetPrototypeOf\]\] ( V )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-setprototypeof-v)
    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        set_immutable_prototype(agent, self.into_object(), prototype)
    }

    /// ### [10.4.6.3 \[\[IsExtensible\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-isextensible)
    fn internal_is_extensible(self, _agent: &mut Agent) -> JsResult<bool> {
        Ok(false)
    }

    /// ### [10.4.6.4 \[\[PreventExtensions\]\] ( )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-preventextensions)
    fn internal_prevent_extensions(self, _agent: &mut Agent) -> JsResult<bool> {
        Ok(true)
    }

    /// 10.4.6.5 \[\[GetOwnProperty\]\] ( P )
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        match property_key {
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return OrdinaryGetOwnProperty(O, P).
                Ok(self
                    .get_backing_object(agent)
                    .and_then(|object| ordinary_get_own_property(agent, object, property_key)))
            }
            // TODO: Check this but it should not be possible to export any
            // integer-valued names.
            PropertyKey::Integer(_) => Ok(None),
            PropertyKey::SmallString(_) | PropertyKey::String(_) => {
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
                    Ok(None)
                } else {
                    // 4. Let value be ? O.[[Get]](P, O).
                    let value = self.internal_get(agent, property_key, self.into_value())?;
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
    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        match property_key {
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return ! OrdinaryDefineOwnProperty(O, P, Desc).
                Ok(self.get_backing_object(agent).map_or(false, |object| {
                    ordinary_define_own_property(agent, object, property_key, property_descriptor)
                        .unwrap()
                }))
            }
            // TODO: Check this but it should not be possible to export any
            // integer-valued names.
            PropertyKey::Integer(_) => Ok(false),
            PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let current be ? O.[[GetOwnProperty]](P).
                let current = self.internal_get_own_property(agent, property_key)?;
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
    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match property_key {
            PropertyKey::Integer(_) => Ok(false),
            PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                let p = match property_key {
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    _ => unreachable!(),
                };
                // 2. Let exports be O.[[Exports]].
                let exports: &[String] = &agent[self].exports;
                // 3. If exports contains P, return true.
                if exports.contains(&p) {
                    Ok(true)
                } else {
                    // 4. Return false.
                    Ok(false)
                }
            }
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return ! OrdinaryHasProperty(O, P).
                Ok(self.get_backing_object(agent).map_or(false, |object| {
                    ordinary_has_property(agent, object, property_key).unwrap()
                }))
            }
        }
    }

    /// ### [10.4.6.8 \[\[Get\]\] ( P, Receiver )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-get-p-receiver)
    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
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
                        ordinary_get(agent, object, property_key, receiver).unwrap()
                    }))
            }
            PropertyKey::Integer(_) => Ok(Value::Undefined),
            PropertyKey::SmallString(_) | PropertyKey::String(_) => {
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
                    Ok(Value::Undefined)
                } else {
                    // 4. Let m be O.[[Module]].
                    let m = &agent[self].cyclic;
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
                            "Could not resolve module",
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
    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        Ok(false)
    }

    /// ### [10.4.6.10 \[\[Delete\]\] ( P )](https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-delete-p)
    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match property_key {
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, then
                // a. Return ! OrdinaryDelete(O, P).
                Ok(self.get_backing_object(agent).map_or(true, |object| {
                    ordinary_delete(agent, object, property_key).unwrap()
                }))
            }
            PropertyKey::Integer(_) => Ok(false),
            PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                let p = match property_key {
                    PropertyKey::String(data) => String::String(data),
                    PropertyKey::SmallString(data) => String::SmallString(data),
                    _ => unreachable!(),
                };
                // 2. Let exports be O.[[Exports]].
                let exports = &agent[self].exports;
                // 3. If exports contains P, return false.
                if exports.contains(&p) {
                    Ok(false)
                } else {
                    // 4. Return true.
                    Ok(true)
                }
            }
        }
    }

    /// ### [10.4.6.11 \[\[OwnPropertyKeys\]\] ( )])(https://tc39.es/ecma262/#sec-module-namespace-exotic-objects-ownpropertykeys)
    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        // 1. Let exports be O.[[Exports]].
        let exports = agent[self]
            .exports
            .iter()
            .map(|string| PropertyKey::from(*string));
        let exports_count = exports.len();
        // 2. Let symbolKeys be OrdinaryOwnPropertyKeys(O).
        let symbol_keys = self
            .get_backing_object(agent)
            .map_or(vec![], |object| ordinary_own_property_keys(agent, object));
        let symbol_keys_count = symbol_keys.len();
        // 3. Return the list-concatenation of exports and symbolKeys.
        let mut own_property_keys = Vec::with_capacity(exports_count + symbol_keys_count);
        exports.for_each(|export_key| own_property_keys.push(export_key));
        symbol_keys
            .iter()
            .for_each(|symbol_key| own_property_keys.push(*symbol_key));
        Ok(own_property_keys)
    }
}
