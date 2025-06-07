// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::{Agent, JsResult, agent::ExceptionType},
        types::{String, Value},
    },
    engine::context::{Bindable, NoGcScope},
};

use super::{DeclarativeEnvironment, DeclarativeEnvironmentRecord, ModuleEnvironment, OuterEnv};

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
    pub(crate) fn create_immutable_binding(self, agent: &mut Agent, name: String) {
        self.into_declarative()
            .create_immutable_binding(agent, name, true);
    }

    /// ### [InitializeBinding(N, V)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing but uninitialized binding in an
    /// Environment Record. The String value N is the text of the bound name.
    /// V is the value for the binding and is a value of any ECMAScript
    /// language type.
    pub(crate) fn initialize_binding(self, agent: &mut Agent, name: String, value: Value) {
        self.into_declarative()
            .initialize_binding(agent, name, value);
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
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        // 1. Assert: S is true.
        debug_assert!(is_strict);
        // 2. Assert: envRec has a binding for N.
        let binding = self.into_declarative().get_binding(agent, name).unwrap();
        // 3. If the binding for N is an indirect binding, then
        //        a. Let M and N2 be the indirection values provided when this binding for N was created.
        //        b. Let targetEnv be M.[[Environment]].
        //        c. If targetEnv is empty, throw a ReferenceError exception.
        //        d. Return ? targetEnv.GetBindingValue(N2, true).
        // 4. If the binding for N in envRec is an uninitialized binding, throw a ReferenceError exception.
        let Some(value) = binding.value else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::ReferenceError,
                "attempted to access uninitialized binding",
                gc,
            ));
        };
        // 5. Return the value currently bound to N in envRec.
        Ok(value.bind(gc))
    }
}
