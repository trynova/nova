use crate::ecmascript::{
    abstract_operations::testing_and_comparison::same_value,
    builtins::ordinary::ordinary_get_own_property,
    execution::{Agent, JsResult},
    scripts_and_modules::module::ModuleIdentifier,
    types::{
        InternalMethods, IntoObject, IntoValue, Object, OrdinaryObjectInternalSlots,
        PropertyDescriptor, PropertyKey, Value,
    },
};

use super::ordinary::{
    ordinary_define_own_property, ordinary_delete, ordinary_get, ordinary_has_property,
    ordinary_own_property_keys, set_immutable_prototype,
};

pub mod data;

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

impl Module {
    fn get_backing_object(self, agent: &mut Agent) -> Option<Object> {
        agent
            .heap
            .get_module(self.0)
            .object_index
            .map(|idx| idx.into())
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
    fn internal_get_prototype_of(self, _agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(None)
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        set_immutable_prototype(agent, self.into_object(), prototype)
    }

    fn internal_is_extensible(self, _agent: &mut Agent) -> JsResult<bool> {
        Ok(false)
    }

    fn internal_prevent_extensions(self, _agent: &mut Agent) -> JsResult<bool> {
        Ok(true)
    }

    /// 10.4.6.5 [[GetOwnProperty]] ( P )
    ///
    /// The [[GetOwnProperty]] internal method of a module namespace exotic
    /// object O takes argument P (a property key) and returns either a normal
    /// completion containing either a Property Descriptor or undefined, or a
    /// throw completion.
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
                // TODO: Actually implement module exports.
                // 2. Let exports be O.[[Exports]].
                // 3. If exports does not contain P, return undefined.
                // 4. Let value be ? O.[[Get]](P, O).
                // 5. Return PropertyDescriptor { [[Value]]: value, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: false }.
                Ok(None)
            }
        }
    }

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

    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        match property_key {
            PropertyKey::Integer(_) => Ok(false),
            PropertyKey::SmallString(_) | PropertyKey::String(_) => {
                // 2. Let exports be O.[[Exports]].
                // 3. If exports contains P, return true.
                // 4. Return false.
                todo!();
            }
            PropertyKey::Symbol(_) => {
                // 1. If P is a Symbol, return ! OrdinaryHasProperty(O, P).
                Ok(self.get_backing_object(agent).map_or(false, |object| {
                    ordinary_has_property(agent, object, property_key).unwrap()
                }))
            }
        }
    }

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
                // 3. If exports does not contain P, return undefined.
                // 4. Let m be O.[[Module]].
                // 5. Let binding be m.ResolveExport(P).
                // 6. Assert: binding is a ResolvedBinding Record.
                // 7. Let targetModule be binding.[[Module]].
                // 8. Assert: targetModule is not undefined.
                // 9. If binding.[[BindingName]] is NAMESPACE, then
                // a. Return GetModuleNamespace(targetModule).
                // 10. Let targetEnv be targetModule.[[Environment]].
                // 11. If targetEnv is EMPTY, throw a ReferenceError exception.
                // 12. Return ? targetEnv.GetBindingValue(binding.[[BindingName]], true).
                todo!()
            }
        }
    }

    fn internal_set(
        self,
        _agent: &mut Agent,
        _property_key: PropertyKey,
        _value: Value,
        _receiver: Value,
    ) -> JsResult<bool> {
        Ok(false)
    }

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
                // 2. Let exports be O.[[Exports]].
                // 3. If exports contains P, return false.
                // 4. Return true.
                Ok(true)
            }
        }
    }

    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        // 1. Let exports be O.[[Exports]].
        // 2. Let symbolKeys be OrdinaryOwnPropertyKeys(O).
        let symbol_keys = self
            .get_backing_object(agent)
            .map_or(vec![], |object| ordinary_own_property_keys(agent, object));
        // 3. Return the list-concatenation of exports and symbolKeys.
        Ok(symbol_keys)
    }
}
