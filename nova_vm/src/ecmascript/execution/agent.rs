// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
        builtins::{control_abstraction_objects::promise_objects::promise_abstract_operations::promise_jobs::{PromiseReactionJob, PromiseResolveThenableJob}, error::ErrorHeapData, promise::Promise},
        scripts_and_modules::ScriptOrModule,
        types::{Function, IntoValue, Reference, String, Symbol, Value},
    },
    heap::{heap_gc::heap_gc, CreateHeapData},
    Heap,
};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Options {
    pub disable_gc: bool,
    pub print_internals: bool,
}

pub type JsResult<T> = std::result::Result<T, JsError>;

#[derive(Debug, Default, Clone, Copy)]
pub struct JsError(Value);

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

pub(crate) enum InnerJob {
    PromiseResolveThenable(PromiseResolveThenableJob),
    PromiseReaction(PromiseReactionJob),
}

pub struct Job {
    pub(crate) realm: Option<RealmIdentifier>,
    pub(crate) inner: InnerJob,
}

impl Job {
    pub fn realm(&self) -> Option<RealmIdentifier> {
        self.realm
    }

    pub fn run(&self, agent: &mut Agent) -> JsResult<()> {
        let mut pushed_context = false;
        if let Some(realm) = self.realm {
            if agent.current_realm_id() != realm {
                agent.execution_context_stack.push(ExecutionContext {
                    ecmascript_code: None,
                    function: None,
                    realm,
                    script_or_module: None,
                });
                pushed_context = true;
            }
        };

        let result = match self.inner {
            InnerJob::PromiseResolveThenable(job) => job.run(agent),
            InnerJob::PromiseReaction(job) => job.run(agent),
        };

        if pushed_context {
            agent.execution_context_stack.pop();
        }

        result
    }
}

pub enum PromiseRejectionTrackerOperation {
    Reject,
    Handle,
}

pub trait HostHooks: std::fmt::Debug {
    /// ### [19.2.1.2 HostEnsureCanCompileStrings ( calleeRealm )](https://tc39.es/ecma262/#sec-hostensurecancompilestrings)
    fn host_ensure_can_compile_strings(&self, _callee_realm: &mut Realm) -> JsResult<()> {
        // The default implementation of HostEnsureCanCompileStrings is to return NormalCompletion(unused).
        Ok(())
    }

    /// ### [20.2.5 HostHasSourceTextAvailable ( func )](https://tc39.es/ecma262/#sec-hosthassourcetextavailable)
    fn host_has_source_text_available(&self, _func: Function) -> bool {
        // The default implementation of HostHasSourceTextAvailable is to return true.
        true
    }

    /// ### [9.5.5 HostEnqueuePromiseJob ( job, realm )](https://tc39.es/ecma262/#sec-hostenqueuepromisejob)
    fn enqueue_promise_job(&self, job: Job);

    /// ### [27.2.1.9 HostPromiseRejectionTracker ( promise, operation )](https://tc39.es/ecma262/#sec-host-promise-rejection-tracker)
    fn promise_rejection_tracker(
        &self,
        _promise: Promise,
        _operation: PromiseRejectionTrackerOperation,
    ) {
        // The default implementation of HostPromiseRejectionTracker is to return unused.
    }
}

pub struct BoxedAgent {
    agent: Box<Agent>,
    root_realms: Vec<RealmIdentifier>,
}

impl BoxedAgent {
    pub fn new(options: Options, host_hooks: &'static dyn HostHooks) -> Self {
        Self {
            agent: Box::new(Agent::new(options, host_hooks)),
            root_realms: Vec::with_capacity(1),
        }
    }

    pub fn with<'agent, F, R>(&'agent mut self, func: F) -> R
    where
        F: FnOnce(&'agent mut Agent, &'agent mut Vec<RealmIdentifier>) -> R,
    {
        func(&mut self.agent, &mut self.root_realms)
    }

    pub fn gc(&mut self) {
        self.agent.gc(&mut self.root_realms);
    }
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
    pub(crate) fn new(options: Options, host_hooks: &'static dyn HostHooks) -> Self {
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

    pub fn create_exception(&mut self, kind: ExceptionType, message: &'static str) -> Value {
        let message = String::from_str(self, message);
        self.heap
            .create(ErrorHeapData::new(kind, Some(message), None))
            .into_value()
    }

    /// ### [5.2.3.2 Throw an Exception](https://tc39.es/ecma262/#sec-throw-an-exception)
    pub fn throw_exception(&mut self, kind: ExceptionType, message: &'static str) -> JsError {
        JsError(self.create_exception(kind, message))
    }

    pub(crate) fn running_execution_context(&self) -> &ExecutionContext {
        self.execution_context_stack.last().unwrap()
    }

    pub(crate) fn running_execution_context_mut(&mut self) -> &mut ExecutionContext {
        self.execution_context_stack.last_mut().unwrap()
    }

    fn gc(&mut self, realm_roots: &mut [RealmIdentifier]) {
        heap_gc(&mut self.heap, realm_roots);
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

    // 3. Let strict be IsStrict(the syntactic production that is being evaluated).
    let strict = agent
        .running_execution_context()
        .ecmascript_code
        .unwrap()
        .is_strict_mode;

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
