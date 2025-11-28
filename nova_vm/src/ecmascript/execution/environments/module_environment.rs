// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;

use crate::{
    ecmascript::{
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, JsError, TryResult},
        },
        scripts_and_modules::module::module_semantics::abstract_module_records::{
            AbstractModule, AbstractModuleSlots,
        },
        types::{String, Value},
    },
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
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
/// Record methods listed in [Table 16](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
/// and share the same specifications for all of those methods except for
/// GetBindingValue, DeleteBinding, HasThisBinding and GetThisBinding. In
/// addition, Module Environment Records support the methods listed in
///# [Table 22](https://tc39.es/ecma262/#table-additional-methods-of-module-environment-records).
///
/// NOTE: There is no data-wise difference between a DeclarativeEnvironment and
/// a ModuleEnvironment, so we treat them exactly the same way.
#[derive(Debug)]
pub struct ModuleEnvironmentRecord {
    /// Module Environment Records support all of the Declarative Environment
    /// Record methods listed in [Table 16](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    /// and share the same specifications for all of those methods except for
    /// GetBindingValue, DeleteBinding, HasThisBinding and GetThisBinding.
    declarative_environment: DeclarativeEnvironment<'static>,
    indirect_bindings: AHashMap<String<'static>, IndirectBinding<'static>>,
}

#[derive(Debug)]
struct IndirectBinding<'a> {
    /// ### \[\[M]]
    ///
    /// Module record which holds the direct binding for \[\[N2]].
    m: AbstractModule<'a>,
    /// ### \[\[N2]]
    ///
    /// Name of the direct binding in \[\[M]].
    n2: String<'a>,
}

impl ModuleEnvironmentRecord {
    fn new(dcl_env: DeclarativeEnvironment) -> Self {
        Self {
            declarative_environment: dcl_env.unbind(),
            indirect_bindings: Default::default(),
        }
    }

    fn has_indirect_binding(&self, name: String) -> bool {
        self.indirect_bindings.contains_key(&name.unbind())
    }

    fn get_indirect_binding(&self, name: String) -> Option<&IndirectBinding<'static>> {
        self.indirect_bindings.get(&name.unbind())
    }
}

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
    agent.heap.alloc_counter += core::mem::size_of::<Option<DeclarativeEnvironmentRecord>>()
        + core::mem::size_of::<Option<ModuleEnvironmentRecord>>();
    // 2. Set env.[[OuterEnv]] to E.
    let declarative_environment = agent
        .heap
        .environments
        .push_declarative_environment(DeclarativeEnvironmentRecord::new(outer_env), gc);
    // 3. Return env.
    agent
        .heap
        .environments
        .push_module_environment(ModuleEnvironmentRecord::new(declarative_environment), gc)
}

impl<'e> ModuleEnvironment<'e> {
    fn get_declarative_env(self, agent: &impl AsRef<Environments>) -> DeclarativeEnvironment<'e> {
        agent
            .as_ref()
            .get_module_environment(self)
            .declarative_environment
    }

    pub(crate) fn get_outer_env(self, agent: &Agent) -> OuterEnv<'e> {
        self.get_declarative_env(agent).get_outer_env(agent)
    }

    /// # [HasBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record has a binding for the String value
    /// N. Return true if it does and false if it does not.
    pub(crate) fn has_binding(self, agent: &impl AsRef<Environments>, name: String) -> bool {
        let env = agent.as_ref().get_module_environment(self);

        env.has_indirect_binding(name) || env.declarative_environment.has_binding(agent, name)
    }

    /// # [CreateMutableBinding(N, D)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If the
    /// Boolean argument D is true the binding may be subsequently deleted.
    pub fn create_mutable_binding(self, agent: &mut Agent, name: String, is_deletable: bool) {
        self.get_declarative_env(agent)
            .create_mutable_binding(agent, name, is_deletable);
    }

    /// # [CreateImmutableBinding(N, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized immutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If S is true
    /// then attempts to set it after it has been initialized will always throw
    /// an exception, regardless of the strict mode setting of operations that
    /// reference that binding.
    pub(crate) fn create_immutable_binding(
        self,
        envs: &mut impl AsMut<Environments>,
        name: String,
    ) {
        let envs = envs.as_mut();
        self.inner_create_immutable_binding(envs, name);
    }

    fn inner_create_immutable_binding(self, envs: &mut Environments, name: String) {
        envs.get_declarative_environment_mut(self.get_declarative_env(envs))
            .create_immutable_binding(name, true);
    }

    /// # [InitializeBinding(N, V)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing but uninitialized binding in an
    /// Environment Record. The String value N is the text of the bound name.
    /// V is the value for the binding and is a value of any ECMAScript
    /// language type.
    pub(crate) fn initialize_binding(
        self,
        envs: &mut impl AsMut<Environments>,
        name: String,
        value: Value,
    ) {
        let envs = envs.as_mut();
        self.inner_initialize_binding(envs, name, value);
    }

    fn inner_initialize_binding(self, envs: &mut Environments, name: String, value: Value) {
        envs.get_declarative_environment_mut(self.get_declarative_env(envs))
            .initialize_binding(name, value);
    }

    /// # [SetMutableBinding(N, V, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
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
        let env_rec = agent.heap.environments.get_module_environment(self);
        if env_rec.has_indirect_binding(name) {
            let error_message = format!(
                "Cannot assign to immutable binding '{}'.",
                name.to_string_lossy(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc));
        }
        env_rec
            .declarative_environment
            .set_mutable_binding(agent, name, value, true, gc)
    }

    pub(crate) fn try_get_binding_value(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'e, '_>,
    ) -> TryResult<'e, Value<'e>> {
        let Some(value) = self.get_binding_value(agent, name, is_strict, gc) else {
            return throw_uninitialized_binding(agent, name, gc).into();
        };
        TryResult::Continue(value)
    }

    pub(crate) fn env_get_binding_value(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'e, '_>,
    ) -> JsResult<'e, Value<'e>> {
        let Some(value) = self.get_binding_value(agent, name, is_strict, gc) else {
            return Err(throw_uninitialized_binding(agent, name, gc));
        };
        Ok(value)
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
    pub(crate) fn get_binding_value(
        self,
        agent: &Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'e, '_>,
    ) -> Option<Value<'e>> {
        // 1. Assert: S is true.
        debug_assert!(is_strict);
        // 2. Assert: envRec has a binding for N.
        debug_assert!(self.has_binding(agent, name));
        // 3. If the binding for N is an indirect binding, then
        let env_rec = agent.heap.environments.get_module_environment(self);
        if let Some(IndirectBinding { m, n2 }) = env_rec.get_indirect_binding(name) {
            // a. Let M and N2 be the indirection values provided when this
            //    binding for N was created.
            // b. Let targetEnv be M.[[Environment]].
            // c. If targetEnv is empty, throw a ReferenceError exception.
            let target_env = m.environment(agent, gc)?;
            // d. Return ? targetEnv.GetBindingValue(N2, true).
            return target_env.get_binding_value(agent, *n2, true, gc);
        }
        let decl_env = env_rec.declarative_environment;
        let binding = agent
            .heap
            .environments
            .get_declarative_environment(decl_env)
            .get_binding(name)
            .unwrap();
        // 4. If the binding for N in envRec is an uninitialized binding, throw
        //    a ReferenceError exception.
        let value = binding.value?;
        // 5. Return the value currently bound to N in envRec.
        Some(value.bind(gc))
    }
}

pub(crate) fn throw_uninitialized_binding<'a>(
    agent: &mut Agent,
    name: String,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    let name = name.to_string_lossy(agent);
    agent.throw_exception(
        ExceptionType::ReferenceError,
        format!("attempted to access uninitialized binding {name}"),
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
    agent: &mut Agent,
    env_rec: ModuleEnvironment,
    n: String,
    m: AbstractModule,
    n2: String,
    gc: NoGcScope,
) {
    // 1. Assert: envRec does not already have a binding for N.
    debug_assert!(!env_rec.has_binding(agent, n));
    let m_environment = m.environment(agent, gc);
    let envs = &mut agent.heap.environments;
    let can_be_direct_binding = if let Some(m_environment) = m_environment {
        // 2. Assert: When M.[[Environment]] is instantiated, it will have a
        //    direct binding for N2.
        let m_decl_env = m_environment.get_declarative_env(envs);
        let m_decl_env = envs.get_declarative_environment_mut(m_decl_env);
        let m_direct_binding = m_decl_env.get_binding(n2);
        let Some(m_direct_binding) = m_direct_binding else {
            unreachable!();
        };
        !m_direct_binding.mutable
    } else {
        // We enter this path when the target module is our current module's
        // parent, ie. this is a circular import. All circular bindings are
        // must be indirect as we cannot know if we're pointing to immutable or
        // mutable data.
        false
    };
    if can_be_direct_binding {
        // Optimisation: references to immutable bindings can be initialised as
        // direct bindings, as the data behind them can never change. The data
        // for the direct binding will be initialised when the module is about
        // to be evaluated.
        env_rec.create_immutable_binding(envs, n);
    } else {
        // 3. Create an immutable indirect binding in envRec for N that
        //    references M and N2 as its target binding and record that the
        //    binding is initialized.
        let created_new = envs
            .get_module_environment_mut(env_rec)
            .indirect_bindings
            .insert(
                n.unbind(),
                IndirectBinding {
                    m: m.unbind(),
                    n2: n2.unbind(),
                },
            )
            .is_none();
        debug_assert!(created_new);
    }
    // 4. Return unused.
}

/// ### [9.1.1.5.5 CreateImportBinding ( envRec, N, M, N2 )](https://tc39.es/ecma262/#sec-createimportbinding)
///
/// Note: this version does not assert that the target module will have a
/// direct binding for the target name. It always creates an indirect binding.
pub(crate) fn create_indirect_import_binding(
    agent: &mut Agent,
    env_rec: ModuleEnvironment,
    n: String,
    m: AbstractModule,
    n2: String,
) {
    // 1. Assert: envRec does not already have a binding for N.
    debug_assert!(!env_rec.has_binding(agent, n));
    let envs = &mut agent.heap.environments;
    // 2. Assert: When M.[[Environment]] is instantiated, it will have a direct
    //    binding for N2.
    // 3. Create an immutable indirect binding in envRec for N that
    //    references M and N2 as its target binding and record that the
    //    binding is initialized.
    let created_new = envs
        .get_module_environment_mut(env_rec)
        .indirect_bindings
        .insert(
            n.unbind(),
            IndirectBinding {
                m: m.unbind(),
                n2: n2.unbind(),
            },
        )
        .is_none();
    debug_assert!(created_new);
    // 4. Return unused.
}

/// ### [9.1.1.5.5 CreateImportBinding ( envRec, N, M, N2 )](https://tc39.es/ecma262/#sec-createimportbinding)
///
/// > NOTE: Performs the initializing of a previously created import binding.
pub(crate) fn initialize_import_binding(
    agent: &mut Agent,
    env_rec: ModuleEnvironment,
    n: String,
    m: AbstractModule,
    n2: String,
    gc: NoGcScope,
) {
    let direct_binding = env_rec.get_declarative_env(agent).get_binding_mut(agent, n);
    let Some(direct_binding) = direct_binding else {
        // Note: if we have indirect binding to name, then it has already been
        // initialized as part of CreateImportBinding.
        return;
    };
    debug_assert!(!direct_binding.mutable);
    debug_assert!(direct_binding.strict);
    let direct_binding_value = &mut direct_binding.value as *mut Option<Value<'static>>;
    let value = m
        .environment(agent, gc)
        .expect("Attempted to access unlinked module's environment")
        .get_declarative_env(agent)
        .get_binding(agent, n2)
        .expect("Direct binding target did not exist")
        .value
        .expect("Attempted to access uninitialized binding");
    // SAFETY: m.environment, get_declarative_env and get_binding both perform
    // no mutation on module environments; the direct_binding_value pointer
    // still points to valid memory and has not been trampled with.
    unsafe { *direct_binding_value = Some(value) };
    // 4. Return unused.
}

impl HeapMarkAndSweep for ModuleEnvironment<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.module_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .module_environments
            .shift_non_zero_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for ModuleEnvironmentRecord {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            declarative_environment,
            indirect_bindings,
        } = self;
        declarative_environment.mark_values(queues);
        indirect_bindings.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            declarative_environment,
            indirect_bindings,
        } = self;
        declarative_environment.sweep_values(compactions);
        indirect_bindings.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for IndirectBinding<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { m, n2 } = self;
        m.mark_values(queues);
        n2.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { m, n2 } = self;
        m.sweep_values(compactions);
        n2.sweep_values(compactions);
    }
}
