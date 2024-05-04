//! ## Notes
//!
//! - This is inspired by and/or copied from Kiesel engine:
//!   Copyright (c) 2023-2024 Linus Groh

use super::{
    environments::get_identifier_reference, EnvironmentIndex, ExecutionContext, Realm,
    RealmIdentifier,
};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_string,
        builtins::{error::ErrorHeapData, promise::Promise},
        scripts_and_modules::ScriptOrModule,
        types::{Function, Reference, String, Symbol, Value},
    },
    heap::indexes::ErrorIndex,
    Heap,
};
use std::{any::Any, collections::HashMap};

#[derive(Debug, Default)]
pub struct Options {
    pub disable_gc: bool,
    pub print_ast: bool,
    pub print_bytecode: bool,
}

pub type JsResult<T> = std::result::Result<T, JsError>;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct JsError(pub(crate) Value);

impl JsError {
    pub(crate) fn new(value: Value) -> Self {
        Self(value)
    }

    pub fn value(self) -> Value {
        self.0
    }

    pub fn to_string(self, agent: &mut Agent) -> String {
        to_string(agent, self.0).unwrap()
    }
}

// #[derive(Debug)]
// pub struct PreAllocated;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromiseRejectionOperation {
    Reject,
    Handle,
}

pub trait HostHooks: std::fmt::Debug {
    fn host_ensure_can_compile_strings(&self, callee_realm: &mut Realm) -> JsResult<()>;
    fn host_has_source_text_available(&self, func: Function) -> bool;
    /// ### [16.2.1.8 HostLoadImportedModule ( referrer, specifier, hostDefined, payload )](https://tc39.es/ecma262/#sec-HostLoadImportedModule)
    ///
    /// The host-defined abstract operation HostLoadImportedModule takes
    /// arguments referrer (a Script Record, a Cyclic Module Record, or a Realm
    /// Record), specifier (a String), hostDefined (anything), and payload (a
    /// GraphLoadingState Record or a PromiseCapability Record) and returns
    /// unused.
    /// 
    /// #### Note
    ///
    /// An example of when referrer can be a Realm Record is in a web browser
    /// host. There, if a user clicks on a control given by
    /// ```html
    /// <button type="button" onclick="import('./foo.mjs')">Click me</button>
    /// ```
    /// there will be no active script or module at the time the `import()`
    /// expression runs. More generally, this can happen in any situation where
    /// the host pushes execution contexts with null ScriptOrModule components
    /// onto the execution context stack.
    ///
    /// An implementation of HostLoadImportedModule must conform to the
    /// following requirements:
    ///
    /// * The host environment must perform
    /// `FinishLoadingImportedModule(referrer, specifier, payload, result)`,
    /// where result is either a normal completion containing the loaded Module
    /// Record or a throw completion, either synchronously or asynchronously.
    /// * If this operation is called multiple times with the same (referrer,
    /// specifier) pair and it performs
    /// `FinishLoadingImportedModule(referrer, specifier, payload, result)`
    /// where result is a normal completion, then it must perform
    /// `FinishLoadingImportedModule(referrer, specifier, payload, result)`
    /// with the same result each time.
    /// * The operation must treat payload as an opaque value to be passed
    /// through to FinishLoadingImportedModule.
    ///
    /// The actual process performed is host-defined, but typically consists of
    /// performing whatever I/O operations are necessary to load the
    /// appropriate Module Record. Multiple different (referrer, specifier)
    /// pairs may map to the same Module Record instance. The actual mapping
    /// semantics is host-defined but typically a normalization process is
    /// applied to specifier as part of the mapping process. A typical
    /// normalization process would include actions such as expansion of
    /// relative and abbreviated path specifiers.
    fn host_load_imported_module(
        &self,
        referrer: (),
        specifier: &str,
        host_defined: Option<Box<dyn Any>>,
        payload: (),
    );
    /// ### [27.2.1.9 HostPromiseRejectionTracker ( promise, operation )](https://tc39.es/ecma262/#sec-host-promise-rejection-tracker)
    ///
    /// The host-defined abstract operation HostPromiseRejectionTracker takes
    /// arguments promise (a Promise) and operation ("reject" or "handle") and
    /// returns unused. It allows host environments to track promise rejections.
    /// 
    /// The default implementation of HostPromiseRejectionTracker is to return
    /// unused.
    ///
    /// #### Note 1
    ///
    /// HostPromiseRejectionTracker is called in two scenarios:
    ///
    /// When a promise is rejected without any handlers, it is called with its
    /// operation argument set to "reject".
    /// When a handler is added to a rejected promise for the first time, it is
    /// called with its operation argument set to "handle".
    ///
    /// A typical implementation of HostPromiseRejectionTracker might try to
    /// notify developers of unhandled rejections, while also being careful to
    /// notify them if such previous notifications are later invalidated by new
    /// handlers being attached.
    ///
    /// #### Note 2
    ///
    /// If operation is "handle", an implementation should not hold a reference
    /// to promise in a way that would interfere with garbage collection. An
    /// implementation may hold a reference to promise if operation is "reject",
    /// since it is expected that rejections will be rare and not on hot code
    /// paths.
    fn host_promise_rejection_tracker(&self, promise: Promise, operation: PromiseRejectionOperation);
}

/// ### [9.7 Agents](https://tc39.es/ecma262/#sec-agents)
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
        &self[id]
    }

    pub fn get_realm_mut(&mut self, id: RealmIdentifier) -> &mut Realm {
        &mut self[id]
    }

    /// ### [5.2.3.2 Throw an Exception](https://tc39.es/ecma262/#sec-throw-an-exception)
    pub fn throw_exception(&mut self, kind: ExceptionType, message: &'static str) -> JsError {
        let message = String::from_str(self, message);
        self.heap
            .errors
            .push(Some(ErrorHeapData::new(kind, Some(message), None)));
        let index = ErrorIndex::last(&self.heap.errors);
        JsError(Value::Error(index))
    }

    pub(crate) fn running_execution_context(&self) -> &ExecutionContext {
        self.execution_context_stack.last().unwrap()
    }

    pub(crate) fn running_execution_context_mut(&mut self) -> &mut ExecutionContext {
        self.execution_context_stack.last_mut().unwrap()
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
    name: String,
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

#[derive(Debug, Clone, Copy)]
pub enum ExceptionType {
    Error,
    AggregateError,
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
}
