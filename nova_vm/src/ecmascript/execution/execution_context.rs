use super::{EnvironmentIndex, PrivateEnvironmentIndex, RealmIdentifier};
use crate::ecmascript::{scripts_and_modules::ScriptOrModule, types::*};

// TODO: Remove this.
pub type ECMAScriptCode = ECMAScriptCodeEvaluationState;

/// ### [code evaluation state](https://tc39.es/ecma262/#table-state-components-for-all-execution-contexts)
///
/// ECMAScript code execution contexts have the additional state components
/// listed in Table 26.
#[derive(Debug)]
pub struct ECMAScriptCodeEvaluationState {
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
}

/// ### [9.4 Execution Contexts](https://tc39.es/ecma262/#sec-execution-contexts)
///
/// An execution context is a specification device that is used to track the
/// runtime evaluation of code by an ECMAScript implementation. At any point in
/// time, there is at most one execution context per agent that is actually
/// executing code. This is known as the agent's running execution context. All
/// references to the running execution context in this specification denote the
/// running execution context of the surrounding agent.
#[derive(Debug)]
pub(crate) struct ExecutionContext {
    /// ### code evaluation state
    ///
    /// Any state needed to perform, suspend, and resume evaluation of the code
    /// associated with this execution context.
    pub ecmascript_code: Option<ECMAScriptCodeEvaluationState>,

    /// ### Function
    ///
    /// If this execution context is evaluating the code of a function object,
    /// then the value of this component is that function object. If the context
    /// is evaluating the code of a Script or Module, the value is null.
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
