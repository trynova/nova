use std::collections::HashMap;

use crate::ecmascript::{
    builtins::module::Module,
    execution::{agent::ExceptionType, Agent, JsResult},
    types::{Object, String, Value},
};

use super::{declarative_environment::Binding, ModuleEnvironmentIndex};

/// ### [9.1.1.5 Module Environment Records](https://tc39.es/ecma262/#sec-module-environment-records)
/// A Module Environment Record is a Declarative Environment Record that is
/// used to represent the outer scope of an ECMAScript Module. In additional to
/// normal mutable and immutable bindings, Module Environment Records also
/// provide immutable import bindings which are bindings that provide indirect
/// access to a target binding that exists in another Environment Record.
///
/// Module Environment Records support all of the Declarative Environment
/// Record methods listed in Table 16 and share the same specifications for all
/// of those methods except for GetBindingValue, DeleteBinding, HasThisBinding
/// and GetThisBinding.
///
/// NOTE: There is no data-wise difference between a DeclarativeEnvironment and
/// a ModuleEnvironment, so we treat them exactly the same way.
#[derive(Default, Debug, Clone)]
pub(crate) struct ModuleEnvironment {
    /// The environment's bindings.
    pub(crate) bindings: HashMap<String, ModuleBinding>,
}

#[derive(Debug, Clone)]
pub(crate) enum ModuleBinding {
    Lexical(Binding),
    Indirect {
        /// Module that this binding references
        module: Module,
        /// Name that this binding references
        name: String,
    },
}

impl ModuleBinding {
    fn is_direct_binding(&self) -> bool {
        matches!(self, ModuleBinding::Lexical(_))
    }
}

impl ModuleEnvironment {
    /// ### [9.1.1.1.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-declarative-environment-records-hasbinding-n)
    pub(super) fn has_binding(&self, name: String) -> bool {
        // 1. If envRec has a binding for N, return true.
        // 2. Return false.
        self.bindings.contains_key(&name)
    }
    /// ### [9.1.1.1.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-declarative-environment-records-createmutablebinding-n-d)
    pub(super) fn create_mutable_binding(&mut self, name: String, is_deletable: bool) {
        // 1. Assert: envRec does not already have a binding for N.
        debug_assert!(!self.has_binding(name));

        // 2. Create a mutable binding in envRec for N and record that it is
        // uninitialized. If D is true, record that the newly created binding
        // may be deleted by a subsequent DeleteBinding call.
        self.bindings.insert(
            name,
            ModuleBinding::Lexical(Binding {
                value: None,
                // TODO: Figure out how/if we should propagate this.
                strict: true,
                mutable: true,
                deletable: is_deletable,
            }),
        );

        // 3. Return UNUSED.
    }
    /// ### [9.1.1.1.3 CreateImmutableBinding ( N, S )](https://tc39.es/ecma262/#sec-declarative-environment-records-createimmutablebinding-n-s)
    pub(super) fn create_immutable_binding(&mut self, name: String, is_strict: bool) {
        // 1. Assert: envRec does not already have a binding for N.
        debug_assert!(!self.has_binding(name));

        // 2. Create an immutable binding in envRec for N and record that it is
        // uninitialized. If S is true, record that the newly created binding is
        // a strict binding.
        self.bindings.insert(
            name,
            ModuleBinding::Lexical(Binding {
                value: None,
                strict: is_strict,
                mutable: false,
                deletable: false,
            }),
        );

        // 3. Return UNUSED.
    }
    /// ### [9.1.1.1.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-declarative-environment-records-initializebinding-n-v)
    pub(super) fn initialize_binding(&mut self, name: String, value: Value) {
        // 1. Assert: envRec must have an uninitialized binding for N.
        let binding = self.bindings.get_mut(&name).unwrap();

        let binding = match binding {
            ModuleBinding::Lexical(binding) => binding,
            ModuleBinding::Indirect { .. } => {
                unreachable!("Should never attempt to initialize indirect bindings")
            }
        };
        debug_assert!(binding.value.is_none());

        // 2. Set the bound value for N in envRec to V.
        // 3. Record that the binding for N in envRec has been initialized.
        // Note: Initialization status of N is determined by the Some/None.
        binding.value = Some(value);

        // 4. Return UNUSED.
    }
}

impl ModuleEnvironmentIndex {
    pub(super) fn heap_data(self, agent: &Agent) -> &ModuleEnvironment {
        agent.heap.environments.get_module_environment(self)
    }

    pub(super) fn heap_data_mut(self, agent: &mut Agent) -> &mut ModuleEnvironment {
        agent.heap.environments.get_module_environment_mut(self)
    }

    pub fn has_binding(self, agent: &Agent, name: String) -> bool {
        let env_rec = self.heap_data(agent);
        // Delegate to heap data record method.
        env_rec.has_binding(name)
    }

    fn has_direct_binding(self, agent: &Agent, name: String) -> bool {
        let env_rec = self.heap_data(agent);
        env_rec
            .bindings
            .get(&name)
            .map_or(false, |binding| binding.is_direct_binding())
    }

    pub fn create_mutable_binding(self, agent: &mut Agent, name: String, is_deletable: bool) {
        let env_rec = self.heap_data_mut(agent);
        // Delegate to heap data record method.
        env_rec.create_mutable_binding(name, is_deletable);
    }

    pub(crate) fn create_immutable_binding(self, agent: &mut Agent, name: String, is_strict: bool) {
        let env_rec = self.heap_data_mut(agent);
        // Delegate to heap data record method.
        env_rec.create_immutable_binding(name, is_strict);
    }

    pub(crate) fn initialize_binding(self, agent: &mut Agent, name: String, value: Value) {
        let env_rec = self.heap_data_mut(agent);
        // Delegate to heap data record method.
        env_rec.initialize_binding(name, value)
    }

    pub(crate) fn set_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        mut is_strict: bool,
    ) -> JsResult<()> {
        let env_rec = self.heap_data_mut(agent);
        // 1. If envRec does not have a binding for N, then
        let Some(binding) = env_rec.bindings.get_mut(&name) else {
            // a. If S is true, throw a ReferenceError exception.
            if is_strict {
                return Err(agent
                    .throw_exception(ExceptionType::ReferenceError, "Identifier is not defined."));
            }

            // b. Perform ! envRec.CreateMutableBinding(N, true).
            env_rec.create_mutable_binding(name, true);

            // c. Perform ! envRec.InitializeBinding(N, V).
            env_rec.initialize_binding(name, value);

            // d. Return UNUSED.
            return Ok(());
        };

        let ModuleBinding::Lexical(binding) = binding else {
            unreachable!("Cannot SetMutableBinding for indirect binding");
        };

        // 2. If the binding for N in envRec is a strict binding, set S to true.
        if binding.strict {
            is_strict = true;
        }

        // 3. If the binding for N in envRec has not yet been initialized, then
        if binding.value.is_none() {
            // a. Throw a ReferenceError exception.
            return Err(
                agent.throw_exception(ExceptionType::ReferenceError, "Identifier is not defined.")
            );
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
                return Err(
                    agent.throw_exception(ExceptionType::TypeError, "Cannot assign to constant.")
                );
            }
        }

        // 6. Return UNUSED.
        Ok(())
    }

    /// ### [9.1.1.5.1 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-module-environment-records-getbindingvalue-n-s)
    ///
    /// The GetBindingValue concrete method of a Module Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or
    /// a throw completion. It returns the value of its bound identifier whose
    /// name is N. However, if the binding is an indirect binding the value of
    /// the target binding is returned. If the binding exists but is
    /// uninitialized a ReferenceError is thrown.
    pub(crate) fn get_binding_value(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
    ) -> JsResult<Value> {
        // 1. Assert: S is true.
        debug_assert!(is_strict);
        // 2. Assert: envRec has a binding for N.
        let binding = self.heap_data(agent).bindings.get(&name);
        let binding = binding.unwrap();
        match binding {
            // 3. If the binding for N is an indirect binding, then
            ModuleBinding::Indirect { module, name } => {
                // a. Let M and N2 be the indirection values provided when this binding for N was created.
                // b. Let targetEnv be M.[[Environment]].
                let target_env = agent[*module].r#abstract.environment;
                match target_env {
                    None => {
                        // c. If targetEnv is empty, throw a ReferenceError exception.
                        Err(agent
                            .throw_exception(ExceptionType::ReferenceError, "Cyclical reference"))
                    }
                    Some(target_env) => {
                        // d. Return ? targetEnv.GetBindingValue(N2, true).
                        target_env.get_binding_value(agent, *name, true)
                    }
                }
            }
            ModuleBinding::Lexical(binding) => {
                // 4. If the binding for N in envRec is an uninitialized binding, throw a ReferenceError exception.
                if binding.value.is_none() {
                    return Err(agent.throw_exception(
                        ExceptionType::ReferenceError,
                        "Accessed uninitialized binding",
                    ));
                }
                // 5. Return the value currently bound to N in envRec.
                Ok(binding.value.unwrap())
            }
        }
    }

    pub(crate) fn delete_binding(self, agent: &mut Agent, _: String) -> bool {
        unreachable!("DeleteBinding should never get called on a Module Environment");
    }

    pub(crate) fn has_this_binding(self) -> bool {
        true
    }

    #[inline(always)]
    pub(crate) fn get_this_binding(self) -> Value {
        Value::Undefined
    }

    pub(crate) fn has_super_binding(self) -> bool {
        false
    }

    pub(crate) fn with_base_object(self) -> Option<Object> {
        None
    }

    /// ### [9.1.1.5.5 CreateImportBinding ( N, M, N2 )](https://tc39.es/ecma262/#sec-createimportbinding)
    ///
    /// The CreateImportBinding concrete method of a Module Environment Record
    /// envRec takes arguments N (a String), M (a Module Record), and N2 (a
    /// String) and returns unused. It creates a new initialized immutable
    /// indirect binding for the name N. A binding must not already exist in
    /// this Environment Record for N. N2 is the name of a binding that exists
    /// in M's Module Environment Record. Accesses to the value of the new
    /// binding will indirectly access the bound value of the target binding.
    pub(crate) fn create_import_binding(
        self,
        agent: &mut Agent,
        name: String,
        module: Module,
        name2: String,
    ) {
        // 1. Assert: envRec does not already have a binding for N.
        debug_assert!(!self.has_binding(agent, name));
        // 2. Assert: When M.[[Environment]] is instantiated, it will have a
        // direct binding for N2.
        debug_assert!({
            let env = agent[module].r#abstract.environment.unwrap();
            env.has_direct_binding(agent, name2)
        });
        // 3. Create an immutable indirect binding in envRec for N that
        // references M and N2 as its target binding and record that the
        // binding is initialized.
        let env_rec = self.heap_data_mut(agent);
        env_rec.bindings.insert(
            name,
            ModuleBinding::Indirect {
                module,
                name: name2,
            },
        );
        // 4. Return unused.
    }
}
