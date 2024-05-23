use crate::ecmascript::{
    builtins::module::Module,
    execution::{agent::ExceptionType, Agent, JsResult},
    types::{String, Value},
};

use super::{DeclarativeEnvironment, DeclarativeEnvironmentIndex};

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
pub(crate) struct ModuleEnvironment(DeclarativeEnvironment);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ModuleEnvironmentIndex(DeclarativeEnvironmentIndex);
impl ModuleEnvironmentIndex {
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
        let binding = self.0.heap_data(agent).bindings.get(&name);
        let binding = binding.unwrap();
        // 3. If the binding for N is an indirect binding, then
        if false {
            // a. Let M and N2 be the indirection values provided when this binding for N was created.
            // b. Let targetEnv be M.[[Environment]].
            // c. If targetEnv is empty, throw a ReferenceError exception.
            // d. Return ? targetEnv.GetBindingValue(N2, true).
            todo!();
        }
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

    pub(crate) fn has_this_binding(self) -> bool {
        true
    }

    pub(crate) fn get_this_binding(self) -> Option<Value> {
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
        debug_assert!(!self.0.has_binding(agent, name));
        // 2. Assert: When M.[[Environment]] is instantiated, it will have a
        // direct binding for N2.
        // 3. Create an immutable indirect binding in envRec for N that
        // references M and N2 as its target binding and record that the
        // binding is initialized.
        // 4. Return unused.
        todo!();
    }
}
