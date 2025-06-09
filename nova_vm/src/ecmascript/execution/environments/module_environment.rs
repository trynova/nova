// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, JsError},
        },
        scripts_and_modules::module::module_semantics::source_text_module_records::{
            SourceTextModule, SourceTextModuleHeap,
        },
        types::{String, Value},
    },
    engine::context::{Bindable, NoGcScope},
};

use super::{
    DeclarativeEnvironment, DeclarativeEnvironmentRecord, Environments, ModuleEnvironment, OuterEnv,
};

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
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ModuleEnvironmentRecord(DeclarativeEnvironmentRecord);

/// 9.1.2.6 NewModuleEnvironment ( E )
///
/// The abstract operation NewModuleEnvironment takes argument E (an
/// Environment Record) and returns a Module Environment Record.
pub(crate) fn new_module_environment<'a>(
    agent: &mut Agent,
    outer_env: OuterEnv,
    gc: NoGcScope<'a, '_>,
) -> ModuleEnvironment<'a> {
    // 1. Let env be a new Module Environment Record containing no bindings.
    agent.heap.alloc_counter += core::mem::size_of::<Option<DeclarativeEnvironmentRecord>>();
    // 2. Set env.[[OuterEnv]] to E.
    let env = agent
        .heap
        .environments
        .push_declarative_environment(DeclarativeEnvironmentRecord::new(outer_env), gc);
    // 3. Return env.
    ModuleEnvironment::from_u32_index(env.into_u32_index())
}

impl<'e> ModuleEnvironment<'e> {
    fn into_declarative(self) -> DeclarativeEnvironment<'e> {
        DeclarativeEnvironment::from_u32_index(self.into_u32_index())
    }

    pub(crate) fn get_outer_env<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> OuterEnv<'a> {
        self.into_declarative().get_outer_env(agent, gc)
    }

    /// ### [HasBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record has a binding for the String value
    /// N. Return true if it does and false if it does not.
    pub(crate) fn has_binding(self, agent: &mut Agent, name: String) -> bool {
        self.into_declarative().has_binding(agent, name)
    }

    /// ### [CreateMutableBinding(N, D)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If the
    /// Boolean argument D is true the binding may be subsequently deleted.
    pub fn create_mutable_binding(self, agent: &mut Agent, name: String, is_deletable: bool) {
        self.into_declarative()
            .create_mutable_binding(agent, name, is_deletable);
    }

    /// ### [CreateImmutableBinding(N, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized immutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If S is true
    /// then attempts to set it after it has been initialized will always throw
    /// an exception, regardless of the strict mode setting of operations that
    /// reference that binding.
    pub(crate) fn create_immutable_binding(self, envs: &mut Environments, name: String) {
        envs.get_declarative_environment_mut(self.into_declarative())
            .create_immutable_binding(name, true);
    }

    /// ### [InitializeBinding(N, V)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing but uninitialized binding in an
    /// Environment Record. The String value N is the text of the bound name.
    /// V is the value for the binding and is a value of any ECMAScript
    /// language type.
    pub(crate) fn initialize_binding(self, envs: &mut Environments, name: String, value: Value) {
        envs.get_declarative_environment_mut(self.into_declarative())
            .initialize_binding(name, value);
    }

    /// ### [SetMutableBinding(N, V, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. V is the
    /// value for the binding and may be a value of any ECMAScript language
    /// type. S is a Boolean flag. If S is true and the binding cannot be set
    /// throw a TypeError exception.
    pub(crate) fn set_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        self.into_declarative()
            .set_mutable_binding(agent, name, value, true, gc)
    }

    /// ### [9.1.1.5.1 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-module-environment-records)
    ///
    /// The GetBindingValue concrete method of a Module Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion. It returns the value of its bound identifier whose
    /// name is N. However, if the binding is an indirect binding the value of
    /// the target binding is returned. If the binding exists but is
    /// uninitialized a ReferenceError is thrown.
    ///
    /// > NOTE: S will always be true because a Module is always strict mode
    /// > code.
    pub(crate) fn get_binding_value<'a>(
        self,
        envs: &mut Environments,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'a, '_>,
    ) -> Option<Value<'a>> {
        // 1. Assert: S is true.
        debug_assert!(is_strict);
        // 2. Assert: envRec has a binding for N.
        let binding = envs
            .get_declarative_environment_mut(self.into_declarative())
            .get_binding(name)
            .unwrap();
        // 3. If the binding for N is an indirect binding, then
        //        a. Let M and N2 be the indirection values provided when this binding for N was created.
        //        b. Let targetEnv be M.[[Environment]].
        //        c. If targetEnv is empty, throw a ReferenceError exception.
        //        d. Return ? targetEnv.GetBindingValue(N2, true).
        // 4. If the binding for N in envRec is an uninitialized binding, throw a ReferenceError exception.
        let Some(value) = binding.value else {
            return None;
        };
        // 5. Return the value currently bound to N in envRec.
        Some(value.bind(gc))
    }
}

pub(crate) fn throw_uninitialized_binding<'a>(
    agent: &mut Agent,
    name: String,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    let name = name.as_str(agent);
    agent.throw_exception(
        ExceptionType::ReferenceError,
        format!("attempted to access uninitialized binding {}", name),
        gc,
    )
}
/// ### [9.1.1.5.5 CreateImportBinding ( envRec, N, M, N2 )](https://tc39.es/ecma262/#sec-createimportbinding)
///
/// The abstract operation CreateImportBinding takes arguments envRec (a
/// Module Environment Record), N (a String), M (a Module Record), and N2
/// (a String) and returns unused. It creates a new initialized immutable
/// indirect binding for the name N. A binding must not already exist in
/// envRec for N. N2 is the name of a binding that exists in M's Module
/// Environment Record. Accesses to the value of the new binding will
/// indirectly access the bound value of the target binding.
pub(crate) fn create_import_binding(
    envs: &mut Environments,
    _modules: &impl AsRef<SourceTextModuleHeap>,
    env_rec: ModuleEnvironment,
    n: String,
    _m: SourceTextModule,
    _n2: String,
    _gc: NoGcScope,
) {
    // let value = m
    //     .environment(modules)
    //     .get_binding_value(envs, n2, true, gc)
    //     .expect("Attempted to access uninitialized value");
    let env_rec = envs.get_declarative_environment_mut(env_rec.into_declarative());
    // 1. Assert: envRec does not already have a binding for N.
    debug_assert!(!env_rec.has_binding(n));
    // 2. Assert: When M.[[Environment]] is instantiated, it will have a direct
    //    binding for N2.
    // 3. Create an immutable indirect binding in envRec for N that references
    //    M and N2 as its target binding and record that the binding is
    //    initialized.
    env_rec.create_immutable_binding(n, true);
    // env_rec.initialize_binding(n, value);
    // 4. Return unused.
}
