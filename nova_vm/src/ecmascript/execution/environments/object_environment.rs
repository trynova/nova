// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{ObjectEnvironmentIndex, OuterEnv};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{define_property_or_throw, get, has_property, set},
            type_conversion::to_boolean,
        },
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{InternalMethods, Object, PropertyDescriptor, PropertyKey, String, Value},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

/// ### [9.1.1.2 Object Environment Records](https://tc39.es/ecma262/#sec-object-environment-records)
///
/// Each Object Environment Record is associated with an object called its
/// binding object. An Object Environment Record binds the set of string
/// identifier names that directly correspond to the property names of its
/// binding object. Property keys that are not strings in the form of an
/// IdentifierName are not included in the set of bound identifiers. Both own
/// and inherited properties are included in the set regardless of the setting
/// of their \[\[Enumerable\]\] attribute. Because properties can be
/// dynamically added and deleted from objects, the set of identifiers bound by
/// an Object Environment Record may potentially change as a side-effect of any
/// operation that adds or deletes properties. Any bindings that are created as
/// a result of such a side-effect are considered to be a mutable binding even
/// if the Writable attribute of the corresponding property is false. Immutable
/// bindings do not exist for Object Environment Records.
#[derive(Debug, Clone)]
pub struct ObjectEnvironment {
    /// ### \[\[BindingObject\]\]
    ///
    /// The binding object of this Environment Record.
    pub(crate) binding_object: Object,

    /// ### \[\[IsWithEnvironment\]\]
    ///
    /// Indicates whether this Environment Record is created for a with
    /// statement.
    is_with_environment: bool,

    /// ### \[\[OuterEnv\]\]
    ///
    /// See [OuterEnv].
    pub(crate) outer_env: OuterEnv,
}

impl ObjectEnvironment {
    /// ### [9.1.2.3 NewObjectEnvironment ( O, W, E )](https://tc39.es/ecma262/#sec-newobjectenvironmenthttps://tc39.es/ecma262/#sec-newobjectenvironment)
    ///
    /// The abstract operation NewObjectEnvironment takes arguments O (an
    /// Object), W (a Boolean), and E (an Environment Record or null) and
    /// returns an Object Environment Record.
    pub(crate) fn new(
        binding_object: Object,
        is_with_environment: bool,
        outer_env: OuterEnv,
    ) -> ObjectEnvironment {
        // 1. Let env be a new Object Environment Record.
        ObjectEnvironment {
            // 2. Set env.[[BindingObject]] to O.
            binding_object,
            // 3. Set env.[[IsWithEnvironment]] to W.
            is_with_environment,
            // 4. Set env.[[OuterEnv]] to E.
            outer_env,
        }
        // 5. Return env.
    }
}

impl HeapMarkAndSweep for ObjectEnvironment {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            binding_object,
            is_with_environment: _,
            outer_env,
        } = self;
        outer_env.mark_values(queues);
        binding_object.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            binding_object,
            is_with_environment: _,
            outer_env,
        } = self;
        outer_env.sweep_values(compactions);
        binding_object.sweep_values(compactions);
    }
}

impl ObjectEnvironmentIndex {
    /// ### [9.1.1.2.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-object-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of an Object Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It determines if its
    /// associated binding object has a property whose name is N.
    pub(crate) fn has_binding(self, agent: &mut Agent, n: String) -> JsResult<bool> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object;
        let is_with_environment = env_rec.is_with_environment;
        let name = PropertyKey::from(n);
        // 2. Let foundBinding be ? HasProperty(bindingObject, N).
        let found_binding = has_property(agent, binding_object, name)?;
        // 3. If foundBinding is false, return false.
        if !found_binding {
            return Ok(false);
        }
        // 4. If envRec.[[IsWithEnvironment]] is false, return true.
        if !is_with_environment {
            return Ok(true);
        }
        // 5. Let unscopables be ? Get(bindingObject, @@unscopables).
        let unscopables = get(
            agent,
            binding_object,
            PropertyKey::Symbol(WellKnownSymbolIndexes::Unscopables.into()),
        )?;
        // 6. If unscopables is an Object, then
        if let Ok(unscopables) = Object::try_from(unscopables) {
            // a. Let blocked be ToBoolean(? Get(unscopables, N)).
            let blocked = get(agent, unscopables, name)?;
            let blocked = to_boolean(agent, blocked);
            // b. If blocked is true, return false.
            Ok(!blocked)
        } else {
            // 7. Return true.
            Ok(true)
        }
    }
    /// ### [9.1.1.2.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-object-environment-records-createmutablebinding-n-d)
    ///
    /// The CreateMutableBinding concrete method of an Object Environment
    /// Record envRec takes arguments N (a String) and D (a Boolean) and
    /// returns either a normal completion containing UNUSED or a throw
    /// completion. It creates in an Environment Record's associated binding
    /// object a property whose name is N and initializes it to the value
    /// undefined. If D is true, the new property's [[Configurable]] attribute
    /// is set to true; otherwise it is set to false.
    pub(crate) fn create_mutable_binding(
        self,
        agent: &mut Agent,
        n: String,
        d: bool,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object;
        // 2. Perform ? DefinePropertyOrThrow(bindingObject, N, PropertyDescriptor { [[Value]]: undefined, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: D }).
        let n = PropertyKey::from(n);
        define_property_or_throw(
            agent,
            binding_object,
            n,
            PropertyDescriptor {
                value: Some(Value::Undefined),
                writable: Some(true),
                get: None,
                set: None,
                enumerable: Some(true),
                configurable: Some(d),
            },
        )?;
        // 3. Return UNUSED.
        Ok(())
        // NOTE
        // Normally envRec will not have a binding for N but if it does, the
        // semantics of DefinePropertyOrThrow may result in an existing binding
        // being replaced or shadowed or cause an abrupt completion to be
        // returned.
    }

    /// ### [9.1.1.2.3 CreateImmutableBinding ( N, S )](https://tc39.es/ecma262/#sec-object-environment-records-createimmutablebinding-n-s)
    pub(crate) fn create_immutable_binding(self, _: &mut Agent, _: String, _: bool) {
        unreachable!("The CreateImmutableBinding concrete method of an Object Environment Record is never used within this specification.")
    }
    /// ### [9.1.1.2.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-object-environment-records-initializebinding-n-v)
    ///
    /// The InitializeBinding concrete method of an Object Environment Record
    /// envRec takes arguments N (a String) and V (an ECMAScript language
    /// value) and returns either a normal completion containing UNUSED or a
    /// throw completion. It is used to set the bound value of the current
    /// binding of the identifier whose name is N to the value V.
    pub(crate) fn initialize_binding(self, agent: &mut Agent, n: String, v: Value) -> JsResult<()> {
        // 1. Perform ? envRec.SetMutableBinding(N, V, false).
        self.set_mutable_binding(agent, n, v, false)?;
        // 2. Return UNUSED.
        Ok(())
        // NOTE
        // In this specification, all uses of CreateMutableBinding for Object
        // Environment Records are immediately followed by a call to
        // InitializeBinding for the same name. Hence, this specification does
        // not explicitly track the initialization state of bindings in Object
        // Environment Records.
    }

    /// ### [9.1.1.2.5 SetMutableBinding ( N, V, S )](https://tc39.es/ecma262/#sec-object-environment-records-setmutablebinding-n-v-s)
    ///
    /// The SetMutableBinding concrete method of an Object Environment Record
    /// envRec takes arguments N (a String), V (an ECMAScript language value),
    /// and S (a Boolean) and returns either a normal completion containing
    /// UNUSED or a throw completion. It attempts to set the value of the
    /// Environment Record's associated binding object's property whose name is
    /// N to the value V. A property named N normally already exists but if it
    /// does not or is not currently writable, error handling is determined by
    /// S.
    pub(crate) fn set_mutable_binding(
        self,
        agent: &mut Agent,
        n: String,
        v: Value,
        s: bool,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object;
        // 2. Let stillExists be ? HasProperty(bindingObject, N).
        let n = PropertyKey::from(n);
        let still_exists = has_property(agent, binding_object, n)?;
        // 3. If stillExists is false and S is true, throw a ReferenceError exception.
        if !still_exists && s {
            let binding_object_repr = binding_object.into_value().string_repr(agent);
            let error_message = format!(
                "Property '{}' does not exist in {}.",
                n.as_display(agent),
                binding_object_repr.as_str(agent)
            );
            Err(agent.throw_exception(ExceptionType::ReferenceError, error_message))
        } else {
            // 4. Perform ? Set(bindingObject, N, V, S).
            set(agent, binding_object, n, v, s)?;
            // 5. Return UNUSED.
            Ok(())
        }
    }
    /// ### [9.1.1.2.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-object-environment-records-getbindingvalue-n-s)
    ///
    /// The GetBindingValue concrete method of an Object Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion. It returns the value of its associated binding
    /// object's property whose name is N. The property should already exist
    /// but if it does not the result depends upon S.
    pub(crate) fn get_binding_value(
        self,
        agent: &mut Agent,
        n: String,
        s: bool,
    ) -> JsResult<Value> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object;
        let name = PropertyKey::from(n);
        // 2. Let value be ? HasProperty(bindingObject, N).
        let value = has_property(agent, binding_object, name)?;
        // 3. If value is false, then
        if !value {
            // a. If S is false, return undefined; otherwise throw a ReferenceError exception.
            if !s {
                Ok(Value::Undefined)
            } else {
                let binding_object_repr = binding_object.into_value().string_repr(agent);
                let error_message = format!(
                    "Property '{}' does not exist in {}.",
                    name.as_display(agent),
                    binding_object_repr.as_str(agent)
                );
                Err(agent.throw_exception(ExceptionType::ReferenceError, error_message))
            }
        } else {
            // 4. Return ? Get(bindingObject, N).
            get(agent, binding_object, name)
        }
    }

    /// ### [9.1.1.2.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-object-environment-records-deletebinding-n)
    ///
    /// The DeleteBinding concrete method of an Object Environment Record
    /// envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It can only
    /// delete bindings that correspond to properties of the environment
    /// object whose [[Configurable]] attribute have the value true.
    pub(crate) fn delete_binding(self, agent: &mut Agent, name: String) -> JsResult<bool> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_boject = env_rec.binding_object;
        let name = PropertyKey::from(name);
        // 2. Return ? bindingObject.[[Delete]](N).
        binding_boject.internal_delete(agent, name)
    }

    /// ### [9.1.1.2.8 HasThisBinding ( )](https://tc39.es/ecma262/#sec-object-environment-records-hasthisbinding)
    ///
    /// The HasThisBinding concrete method of an Object Environment Record
    /// envRec takes no arguments and returns false.
    pub(crate) fn has_this_binding(&self) -> bool {
        // 1. Return false.
        false
        // NOTE
        // Object Environment Records do not provide a this binding.
    }

    /// ### [9.1.1.2.9 HasSuperBinding ( )](https://tc39.es/ecma262/#sec-object-environment-records-hassuperbinding)
    ///
    /// The HasSuperBinding concrete method of an Object Environment Record
    /// envRec takes no arguments and returns false.
    pub(crate) fn has_super_binding(&self) -> bool {
        // 1. Return false.
        false
        // NOTE
        // Object Environment Records do not provide a super binding.
    }

    /// ### [9.1.1.2.10 WithBaseObject ( )](https://tc39.es/ecma262/#sec-object-environment-records-withbaseobject)
    ///
    /// The WithBaseObject concrete method of an Object Environment Record
    /// envRec takes no arguments and returns an Object or undefined.
    pub(crate) fn with_base_object(self, agent: &Agent) -> Option<Object> {
        let env_rec = &agent[self];
        // 1. If envRec.[[IsWithEnvironment]] is true, return envRec.[[BindingObject]].
        if env_rec.is_with_environment {
            Some(env_rec.binding_object)
        } else {
            // 2. Otherwise, return undefined.
            None
        }
    }
}

impl HeapMarkAndSweep for ObjectEnvironmentIndex {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.object_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = Self::from_u32_index(
            self_index
                - compactions
                    .object_environments
                    .get_shift_for_index(self_index),
        );
    }
}
