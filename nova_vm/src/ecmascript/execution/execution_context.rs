// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{Agent, EnvironmentIndex, PrivateEnvironmentIndex, RealmIdentifier};
use crate::{
    ecmascript::{
        scripts_and_modules::{source_code::SourceCode, ScriptOrModule},
        types::*,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

// TODO: Remove this.
pub(crate) type ECMAScriptCode = ECMAScriptCodeEvaluationState;

/// ### [code evaluation state](https://tc39.es/ecma262/#table-state-components-for-all-execution-contexts)
///
/// ECMAScript code execution contexts have the additional state components
/// listed in Table 26.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ECMAScriptCodeEvaluationState {
    /// ### LexicalEnvironment
    ///
    /// Identifies the Environment Record used to resolve identifier references
    /// made by code within this execution context.
    pub(crate) lexical_environment: EnvironmentIndex,

    /// ### VariableEnvironment
    ///
    /// Identifies the Environment Record that holds bindings created by
    /// VariableStatements within this execution context.
    pub(crate) variable_environment: EnvironmentIndex,

    /// ### PrivateEnvironment
    ///
    /// Identifies the PrivateEnvironment Record that holds Private Names
    /// created by ClassElements in the nearest containing class. null if there
    /// is no containing class.
    pub(crate) private_environment: Option<PrivateEnvironmentIndex>,

    /// Although the spec does not track this here, we also use
    /// [`ECMAScriptCodeEvaluationState`] to track whether some ECMAScript code
    /// is in strict mode.
    pub(crate) is_strict_mode: bool,

    /// Nova-specific piece of data that identifiers in which source code the
    /// currently evaluated code was defined in. Note that this is also
    /// defined for builtin functions: A builtin function's source code will
    /// point to the source code that called it.
    pub(crate) source_code: SourceCode,
}

/// ### [9.4 Execution Contexts](https://tc39.es/ecma262/#sec-execution-contexts)
///
/// An execution context is a specification device that is used to track the
/// runtime evaluation of code by an ECMAScript implementation. At any point in
/// time, there is at most one execution context per agent that is actually
/// executing code. This is known as the agent's running execution context. All
/// references to the running execution context in this specification denote
/// the running execution context of the surrounding agent.
#[derive(Debug, Clone)]
pub(crate) struct ExecutionContext {
    /// ### code evaluation state
    ///
    /// Any state needed to perform, suspend, and resume evaluation of the code
    /// associated with this execution context.
    pub ecmascript_code: Option<ECMAScriptCodeEvaluationState>,

    /// ### Function
    ///
    /// If this execution context is evaluating the code of a function object,
    /// then the value of this component is that function object. If the
    /// context is evaluating the code of a Script or Module, the value is
    /// null.
    pub function: Option<Function>,

    /// ### Realm
    ///
    /// The Realm Record from which associated code accesses ECMAScript
    /// resources.
    pub realm: RealmIdentifier,

    /// ### ScriptOrModule
    ///
    /// The Module Record or Script Record from which associated code
    /// originates. If there is no originating script or module, as is the case
    /// for the original execution context created in
    /// InitializeHostDefinedRealm, the value is null.
    pub script_or_module: Option<ScriptOrModule>,
}

impl ExecutionContext {
    pub(crate) fn suspend(&self) {
        // TODO: What does this actually mean in the end?
    }
}

impl HeapMarkAndSweep for ECMAScriptCodeEvaluationState {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            lexical_environment,
            variable_environment,
            private_environment,
            is_strict_mode: _,
            source_code,
        } = self;
        lexical_environment.mark_values(queues);
        variable_environment.mark_values(queues);
        private_environment.mark_values(queues);
        source_code.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            lexical_environment,
            variable_environment,
            private_environment,
            is_strict_mode: _,
            source_code,
        } = self;
        lexical_environment.sweep_values(compactions);
        variable_environment.sweep_values(compactions);
        private_environment.sweep_values(compactions);
        source_code.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for ExecutionContext {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            ecmascript_code,
            function,
            realm,
            script_or_module,
        } = self;
        ecmascript_code.mark_values(queues);
        function.mark_values(queues);
        realm.mark_values(queues);
        script_or_module.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            ecmascript_code,
            function,
            realm,
            script_or_module,
        } = self;
        ecmascript_code.sweep_values(compactions);
        function.sweep_values(compactions);
        realm.sweep_values(compactions);
        script_or_module.sweep_values(compactions);
    }
}

/// ### [9.4.6 GetGlobalObject ( )](https://tc39.es/ecma262/#sec-getglobalobject)
///
/// The abstract operation GetGlobalObject takes no arguments and returns an
/// Object. It returns the global object used by the currently running
/// execution context.
pub(crate) fn get_global_object(agent: &Agent) -> Object {
    // 1. Let currentRealm be the current Realm Record.
    let current_realm = agent.current_realm();
    // 2. Return currentRealm.[[GlobalObject]].
    current_realm.global_object
}
