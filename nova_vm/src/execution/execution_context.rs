use super::{Environment, PrivateEnvironment};
use crate::{execution::Realm, language::Script, types::*};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct Module;

#[derive(Debug, Clone)]
pub enum ScriptOrModule<'ctx, 'host> {
    Script(Rc<RefCell<Script<'ctx, 'host>>>),
    Module(Rc<RefCell<Module>>),
}

#[derive(Debug)]
pub struct ECMAScriptCode {
    /// LexicalEnvironment
    pub lexical_environment: Environment,

    /// VariableEnvironment
    pub variable_environment: Environment,

    /// PrivateEnvironment
    pub private_environment: Option<Rc<RefCell<PrivateEnvironment>>>,
}

/// 9.4 Execution Contexts
/// https://tc39.es/ecma262/#sec-execution-contexts
#[derive(Debug)]
pub struct ExecutionContext<'ctx, 'host> {
    /// Function
    ///
    /// > If this execution context is evaluating the code of a function object, then the value
    /// > of this component is that function object. If the context is evaluating the code of
    /// > a *Script* or *Module*, the value is **null** (here represented by None).
    pub function: Option<Function>,

    /// Realm
    pub realm: Rc<RefCell<Realm<'ctx, 'host>>>,

    /// ScriptOrModule
    pub script_or_module: Option<ScriptOrModule<'ctx, 'host>>,

    /// ECMAScript code execution contexts have the additional state components listed in Table 26.
    /// https://tc39.es/ecma262/#ecmascript-code-execution-context
    pub ecmascript_code: Option<ECMAScriptCode>,
}
