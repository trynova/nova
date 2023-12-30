use super::{
    environments::get_identifier_reference, EnvironmentIndex, ExecutionContext, Realm,
    RealmIdentifier,
};
use crate::{
    ecmascript::{
        scripts_and_modules::ScriptOrModule,
        types::{Function, Reference, Symbol, Value},
    },
    Heap,
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Options {
    pub disable_gc: bool,
    pub print_ast: bool,
    pub print_bytecode: bool,
}

pub type JsResult<T> = std::result::Result<T, JsError>;

#[derive(Debug, Default, Clone, Copy)]
pub struct JsError {}

// #[derive(Debug)]
// pub struct PreAllocated;

pub trait HostHooks: std::fmt::Debug {
    fn host_ensure_can_compile_strings(&self, callee_realm: &mut Realm) -> JsResult<()>;
    fn host_has_source_text_available(&self, func: Function) -> bool;
}

/// 9.7 Agents
/// https://tc39.es/ecma262/#sec-agents
#[derive(Debug)]
pub struct Agent {
    pub(crate) heap: Heap,
    pub(crate) options: Options,
    // pre_allocated: PreAllocated,
    pub(crate) exception: Option<Value>,
    pub(crate) symbol_id: usize,
    pub(crate) global_symbol_registry: HashMap<&'static str, Symbol>,
    pub(crate) host_hooks: &'static dyn HostHooks,
    pub(crate) execution_context_stack: Vec<ExecutionContext>,
}

impl Agent {
    pub fn new(options: Options, host_hooks: &'static dyn HostHooks) -> Self {
        Self {
            heap: Heap::new(),
            options,
            exception: None,
            symbol_id: 0,
            global_symbol_registry: HashMap::new(),
            host_hooks,
            execution_context_stack: Vec::new(),
        }
    }

    pub fn current_realm_id(&self) -> RealmIdentifier {
        self.execution_context_stack.last().unwrap().realm
    }

    pub fn current_realm(&self) -> &Realm {
        self.get_realm(self.current_realm_id())
    }

    pub fn current_realm_mut(&mut self) -> &mut Realm {
        self.get_realm_mut(self.current_realm_id())
    }

    pub fn get_realm(&self, id: RealmIdentifier) -> &Realm {
        self.heap.get_realm(id)
    }

    pub fn get_realm_mut(&mut self, id: RealmIdentifier) -> &mut Realm {
        self.heap.get_realm_mut(id)
    }

    /// 5.2.3.2 Throw an Exception
    /// https://tc39.es/ecma262/#sec-throw-an-exception
    pub fn throw_exception(&mut self, kind: ExceptionType, message: &'static str) -> JsError {
        todo!("Uncaught {kind:?}: {message}")
    }

    pub(crate) fn running_execution_context(&self) -> &ExecutionContext {
        self.execution_context_stack.last().unwrap()
    }
}

/// ### [9.4.1 GetActiveScriptOrModule ()](https://tc39.es/ecma262/#sec-getactivescriptormodule)
///
/// The abstract operation GetActiveScriptOrModule takes no arguments and
/// returns a Script Record, a Module Record, or null. It is used to determine
/// the running script or module, based on the running execution context.
pub(crate) fn get_active_script_or_module(agent: &mut Agent) -> Option<ScriptOrModule> {
    if agent.execution_context_stack.is_empty() {
        return None;
    }
    let ec = agent
        .execution_context_stack
        .iter()
        .rev()
        .find(|context| context.script_or_module.is_some());
    ec.map(|context| context.script_or_module.unwrap())
}

/// ### [9.4.2 ResolveBinding ( name \[ , env \] )](https://tc39.es/ecma262/#sec-resolvebinding)
///
/// The abstract operation ResolveBinding takes argument name (a String) and
/// optional argument env (an Environment Record or undefined) and returns
/// either a normal completion containing a Reference Record or a throw
/// completion. It is used to determine the binding of name. env can be used to
/// explicitly provide the Environment Record that is to be searched for the
/// binding.
pub(crate) fn resolve_binding(
    agent: &mut Agent,
    name: &str,
    env: Option<EnvironmentIndex>,
) -> JsResult<Reference> {
    let env = env.unwrap_or_else(|| {
        // 1. If env is not present or env is undefined, then
        //    a. Set env to the running execution context's LexicalEnvironment.
        agent
            .running_execution_context()
            .ecmascript_code
            .as_ref()
            .unwrap()
            .lexical_environment
    });

    // 2. Assert: env is an Environment Record.
    // Implicit from env's type.

    // 3. If the source text matched by the syntactic production that is being
    //    evaluated is contained in strict mode code, let strict be true; else
    //    let strict be false.
    // TODO: Implement correctly.
    let strict = false;

    // 4. Return ? GetIdentifierReference(env, name, strict).
    get_identifier_reference(agent, Some(env), name, strict)
}

#[derive(Debug)]
pub enum ExceptionType {
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
}
