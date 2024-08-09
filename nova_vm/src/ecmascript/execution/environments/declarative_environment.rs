// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;

use super::{DeclarativeEnvironmentIndex, OuterEnv};
use crate::{
    ecmascript::{
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{Object, String, Value},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

/// ### [9.1.1.1 Declarative Environment Records](https://tc39.es/ecma262/#sec-declarative-environment-records)
///
/// A Declarative Environment Record is used to define the effect of ECMAScript
/// language syntactic elements such as FunctionDeclarations,
/// VariableDeclarations, and Catch clauses that directly associate identifier
/// bindings with ECMAScript language values.
#[derive(Debug, Clone)]
pub(crate) struct DeclarativeEnvironment<'gen> {
    /// ### \[\[OuterEnv\]\]
    ///
    /// See [OuterEnv].
    pub(crate) outer_env: OuterEnv<'gen>,

    /// The environment's bindings.
    pub(crate) bindings: AHashMap<String<'gen>, Binding<'gen>>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Binding<'gen> {
    pub(crate) value: Option<Value<'gen>>,
    // TODO: Pack these into bitfields.
    pub(super) strict: bool,
    pub(super) mutable: bool,
    pub(super) deletable: bool,
}

impl<'gen> DeclarativeEnvironment<'gen> {
    /// ### [9.1.2.2 NewDeclarativeEnvironment ( E )](https://tc39.es/ecma262/#sec-newdeclarativeenvironment)
    ///
    /// The abstract operation NewDeclarativeEnvironment takes argument E (an
    /// Environment Record or null) and returns a Declarative Environment
    /// Record.
    pub(crate) fn new(outer_env: OuterEnv<'gen>) -> DeclarativeEnvironment<'gen> {
        // 1. Let env be a new Declarative Environment Record containing no bindings.
        // 2. Set env.[[OuterEnv]] to E.
        // 3. Return env.
        DeclarativeEnvironment {
            outer_env,
            bindings: AHashMap::default(),
        }
    }

    /// ### [9.1.1.1.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-declarative-environment-records-hasbinding-n)
    pub(super) fn has_binding(&self, name: String<'gen>) -> bool {
        // 1. If envRec has a binding for N, return true.
        // 2. Return false.
        self.bindings.contains_key(&name)
    }
    /// ### [9.1.1.1.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-declarative-environment-records-createmutablebinding-n-d)
    pub(super) fn create_mutable_binding(&mut self, name: String<'gen>, is_deletable: bool) {
        // 1. Assert: envRec does not already have a binding for N.
        debug_assert!(!self.has_binding(name));

        // 2. Create a mutable binding in envRec for N and record that it is
        // uninitialized. If D is true, record that the newly created binding
        // may be deleted by a subsequent DeleteBinding call.
        self.bindings.insert(
            name,
            Binding {
                value: None,
                // Strictness only seems to matter for immutable bindings.
                strict: false,
                mutable: true,
                deletable: is_deletable,
            },
        );

        // 3. Return UNUSED.
    }
    /// ### [9.1.1.1.3 CreateImmutableBinding ( N, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-createimmutablebinding-n-s)
    pub(super) fn create_immutable_binding(&mut self, name: String<'gen>, is_strict: bool) {
        // 1. Assert: envRec does not already have a binding for N.
        debug_assert!(!self.has_binding(name));

        // 2. Create an immutable binding in envRec for N and record that it is
        // uninitialized. If S is true, record that the newly created binding is
        // a strict binding.
        self.bindings.insert(
            name,
            Binding {
                value: None,
                strict: is_strict,
                mutable: false,
                deletable: false,
            },
        );

        // 3. Return UNUSED.
    }
    /// ### [9.1.1.1.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-declarative-environment-records-initializebinding-n-v)
    pub(super) fn initialize_binding(&mut self, name: String<'gen>, value: Value<'gen>) {
        // 1. Assert: envRec must have an uninitialized binding for N.
        let binding = self.bindings.get_mut(&name).unwrap();

        // 2. Set the bound value for N in envRec to V.
        // 3. Record that the binding for N in envRec has been initialized.
        // Note: Initialization status of N is determined by the Some/None.
        binding.value = Some(value);

        // 4. Return UNUSED.
    }

    /// ### [9.1.1.1.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-getbindingvalue-n-s)
    pub(super) fn get_binding_value(&self, name: String<'gen>, _is_strict: bool) -> Option<Value<'gen>> {
        // 1. Assert: envRec has a binding for N.
        let binding = self.bindings.get(&name).unwrap();

        // 2. If the binding for N in envRec is an uninitialized binding, throw
        // a ReferenceError exception.
        let Some(value) = binding.value else {
            // Custom handling: Return None and let the caller handle throwing
            // an error.
            return None;
        };

        // 3. Return the value currently bound to N in envRec.
        Some(value)
    }

    /// ### [9.1.1.1.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-declarative-environment-records-deletebinding-n)
    pub(super) fn delete_binding(&mut self, name: String<'gen>) -> bool {
        // 1. Assert: envRec has a binding for N.
        let binding = self.bindings.get(&name).unwrap();

        // 2. If the binding for N in envRec cannot be deleted, return false.
        if !binding.deletable {
            return false;
        }

        // 3. Remove the binding for N from envRec.
        self.bindings.remove(&name);

        // 4. Return true.
        true
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for DeclarativeEnvironment<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        self.outer_env.mark_values(queues);
        for binding in self.bindings.values() {
            binding.value.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.outer_env.sweep_values(compactions);
        for binding in self.bindings.values_mut() {
            binding.value.sweep_values(compactions);
        }
        let keys = self.bindings.keys().copied().collect::<Box<[_]>>();
        for key in keys.iter() {
            let mut new_key = *key;
            new_key.sweep_values(compactions);
            if *key != new_key {
                let mut binding = self.bindings.remove(key).unwrap();
                binding.value.sweep_values(compactions);
                self.bindings.insert(new_key, binding);
            }
        }
    }
}

impl<'gen> DeclarativeEnvironmentIndex<'gen> {
    /// ### [9.1.1.1.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-declarative-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of a Declarative Environment Record
    /// envRec takes argument N (a String) and returns a normal completion
    /// containing a Boolean. It determines if the argument identifier is one
    /// of the identifiers bound by the record.
    pub fn has_binding(self, agent: &Agent<'gen>, name: String<'gen>) -> bool {
        let env_rec = &agent[self];
        // Delegate to heap data record method.
        env_rec.has_binding(name)
    }

    /// ### [9.1.1.1.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-declarative-environment-records-createmutablebinding-n-d)
    ///
    /// The CreateMutableBinding concrete method of a Declarative Environment
    /// Record envRec takes arguments N (a String) and D (a Boolean) and
    /// returns a normal completion containing UNUSED. It creates a new mutable
    /// binding for the name N that is uninitialized. A binding must not
    /// already exist in this Environment Record for N. If D is true, the new
    /// binding is marked as being subject to deletion.
    pub fn create_mutable_binding(self, agent: &mut Agent<'gen>, name: String<'gen>, is_deletable: bool) {
        let env_rec = &mut agent[self];
        // Delegate to heap data record method.
        env_rec.create_mutable_binding(name, is_deletable);
    }

    /// ### [9.1.1.1.3 CreateImmutableBinding ( N, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-createimmutablebinding-n-s)
    ///
    /// The CreateImmutableBinding concrete method of a Declarative Environment
    /// Record envRec takes arguments N (a String) and S (a Boolean) and
    /// returns a normal completion containing UNUSED. It creates a new
    /// immutable binding for the name N that is uninitialized. A binding must
    /// not already exist in this Environment Record for N. If S is true, the
    /// new binding is marked as a strict binding.
    pub(crate) fn create_immutable_binding(self, agent: &mut Agent<'gen>, name: String<'gen>, is_strict: bool) {
        let env_rec = &mut agent[self];
        // Delegate to heap data record method.
        env_rec.create_immutable_binding(name, is_strict);
    }

    /// ### [9.1.1.1.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-declarative-environment-records-initializebinding-n-v)
    ///
    /// The InitializeBinding concrete method of a Declarative Environment
    /// Record envRec takes arguments N (a String) and V (an ECMAScript
    /// language value) and returns a normal completion containing UNUSED. It
    /// is used to set the bound value of the current binding of the identifier
    /// whose name is N to the value V. An uninitialized binding for N must
    /// already exist.
    pub(crate) fn initialize_binding(self, agent: &mut Agent<'gen>, name: String<'gen>, value: Value<'gen>) {
        let env_rec = &mut agent[self];
        // Delegate to heap data record method.
        env_rec.initialize_binding(name, value)
    }

    /// ### [9.1.1.1.5 SetMutableBinding ( N, V, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-setmutablebinding-n-v-s)
    ///
    /// The SetMutableBinding concrete method of a Declarative Environment
    /// Record envRec takes arguments N (a String), V (an ECMAScript language
    /// value), and S (a Boolean) and returns either a normal completion
    /// containing UNUSED or a throw completion. It attempts to change the
    /// bound value of the current binding of the identifier whose name is N to
    /// the value V. A binding for N normally already exists, but in rare cases
    /// it may not. If the binding is an immutable binding, a TypeError is
    /// thrown if S is true.
    pub(crate) fn set_mutable_binding(
        self,
        agent: &mut Agent<'gen>,
        name: String<'gen>,
        value: Value<'gen>,
        mut is_strict: bool,
    ) -> JsResult<'gen, ()> {
        let env_rec = &mut agent[self];
        // 1. If envRec does not have a binding for N, then
        let Some(binding) = env_rec.bindings.get_mut(&name) else {
            // a. If S is true, throw a ReferenceError exception.
            if is_strict {
                let error_message = format!("Identifier '{}' does not exist.", name.as_str(agent));
                return Err(agent.throw_exception(ExceptionType::ReferenceError, error_message));
            }

            // b. Perform ! envRec.CreateMutableBinding(N, true).
            env_rec.create_mutable_binding(name, true);

            // c. Perform ! envRec.InitializeBinding(N, V).
            env_rec.initialize_binding(name, value);

            // d. Return UNUSED.
            return Ok(());
        };

        // 2. If the binding for N in envRec is a strict binding, set S to true.
        if binding.strict {
            is_strict = true;
        }

        // 3. If the binding for N in envRec has not yet been initialized, then
        if binding.value.is_none() {
            // a. Throw a ReferenceError exception.
            let error_message = format!(
                "Identifier '{}' has not been initialized.",
                name.as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::ReferenceError, error_message));
        }

        // 4. Else if the binding for N in envRec is a mutable binding, then
        if binding.mutable {
            // a. Change its bound value to V.
            binding.value = Some(value);
        }
        // 5. Else,
        else {
            // a. Assert: This is an attempt to change the value of an immutable binding.
            debug_assert!(!binding.mutable);

            // b. If S is true, throw a TypeError exception.
            if is_strict {
                let error_message = format!(
                    "Cannot assign to immutable identifier '{}' in strict mode.",
                    name.as_str(agent)
                );
                return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
            }
        }

        // 6. Return UNUSED.
        Ok(())
    }

    /// ### [9.1.1.1.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-getbindingvalue-n-s)
    ///
    /// The GetBindingValue concrete method of a Declarative Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion. It returns the value of its bound identifier whose
    /// name is N. If the binding exists but is uninitialized a ReferenceError
    /// is thrown, regardless of the value of S.
    pub(crate) fn get_binding_value(
        self,
        agent: &mut Agent<'gen>,
        name: String<'gen>,
        is_strict: bool,
    ) -> JsResult<'gen, Value<'gen>> {
        let env_rec = &agent[self];
        // Delegate to heap data record method.
        env_rec.get_binding_value(name, is_strict).map_or_else(
            || {
                // 2. If the binding for N in envRec is an uninitialized binding, throw
                // a ReferenceError exception.
                let error_message = format!("Identifier '{}' does not exist.", name.as_str(agent));
                Err(agent.throw_exception(ExceptionType::ReferenceError, error_message))
            },
            Ok,
        )
    }

    /// ### [9.1.1.1.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-declarative-environment-records-deletebinding-n)
    ///
    /// The DeleteBinding concrete method of a Declarative Environment Record
    /// envRec takes argument N (a String) and returns a normal completion
    /// containing a Boolean. It can only delete bindings that have been
    /// explicitly designated as being subject to deletion.
    pub(crate) fn delete_binding(self, agent: &mut Agent<'gen>, name: String<'gen>) -> bool {
        let env_rec = &mut agent[self];
        // Delegate to heap data record method.
        env_rec.delete_binding(name)
    }

    /// ### [9.1.1.1.8 HasThisBinding ( )](https://tc39.es/ecma262/#sec-declarative-environment-records-hasthisbinding)
    ///
    /// The HasThisBinding concrete method of a Declarative Environment Record
    /// envRec takes no arguments and returns false.
    pub(crate) fn has_this_binding(self) -> bool {
        // 1. Return false.
        false
    }

    /// ### [9.1.1.1.9 HasSuperBinding ( )](https://tc39.es/ecma262/#sec-declarative-environment-records-hassuperbinding)
    ///
    /// The HasSuperBinding concrete method of a Declarative Environment Record
    /// envRec takes no arguments and returns false.
    pub(crate) fn has_super_binding(self) -> bool {
        // 1. Return false.
        false
    }

    /// ### [9.1.1.1.10 WithBaseObject ( )](https://tc39.es/ecma262/#sec-declarative-environment-records-withbaseobject)
    ///
    /// The WithBaseObject concrete method of a Declarative Environment Record
    /// envRec takes no arguments and returns undefined.
    pub(crate) fn with_base_object(self) -> Option<Object<'gen>> {
        // 1. Return undefined.
        None
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for DeclarativeEnvironmentIndex<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.declarative_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = Self::from_u32_index(
            self_index
                - compactions
                    .declarative_environments
                    .get_shift_for_index(self_index),
        );
    }
}

/// ### [9.1.2.2 NewDeclarativeEnvironment ( E )](https://tc39.es/ecma262/#sec-newdeclarativeenvironment)
///
/// The abstract operation NewDeclarativeEnvironment takes argument E (an
/// Environment Record or null) and returns a Declarative Environment
/// Record.
pub(crate) fn new_declarative_environment(
    agent: &mut Agent<'gen>,
    outer_env: OuterEnv,
) -> DeclarativeEnvironmentIndex {
    // 1. Let env be a new Declarative Environment Record containing no bindings.
    // 2. Set env.[[OuterEnv]] to E.
    agent
        .heap
        .environments
        .push_declarative_environment(DeclarativeEnvironment::new(outer_env));
    // 3. Return env.
    DeclarativeEnvironmentIndex::last(&agent.heap.environments.declarative)
}
