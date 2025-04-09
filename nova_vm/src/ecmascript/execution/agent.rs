// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## Notes
//!
//! - This is inspired by and/or copied from Kiesel engine:
//!   Copyright (c) 2023-2024 Linus Groh

use ahash::AHashMap;

use super::{
    environments::{get_identifier_reference, try_get_identifier_reference}, initialize_default_realm, initialize_host_defined_realm, Environment, ExecutionContext, GlobalEnvironment, PrivateEnvironment, Realm, RealmIdentifier
};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_string,
        builtins::{control_abstraction_objects::promise_objects::promise_abstract_operations::promise_jobs::{PromiseReactionJob, PromiseResolveThenableJob}, error::ErrorHeapData, promise::Promise},
        scripts_and_modules::{script::{parse_script, script_evaluation}, source_code::SourceCode, ScriptOrModule},
        types::{Function, IntoValue, Object, Reference, String, Symbol, Value},
    }, engine::{context::{Bindable, GcScope, NoGcScope}, rootable::{HeapRootCollectionData, HeapRootData}, TryResult, Vm}, heap::{heap_gc::heap_gc, CreateHeapData, HeapMarkAndSweep, PrimitiveHeapIndexable}, Heap
};
use core::{any::Any, cell::RefCell, ptr::NonNull};

#[derive(Debug, Default)]
pub struct Options {
    pub disable_gc: bool,
    pub print_internals: bool,
}

pub type JsResult<T> = core::result::Result<T, JsError>;

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct JsError(Value<'static>);

impl JsError {
    pub(crate) fn new(value: Value<'static>) -> Self {
        Self(value)
    }

    pub fn value(self) -> Value<'static> {
        self.0
    }

    pub fn to_string<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> String<'gc> {
        to_string(agent, self.0, gc).unwrap()
    }
}

// SAFETY: Property implemented as a recursive bind.
unsafe impl Bindable for JsError {
    type Of<'a> = JsError;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        self
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        self
    }
}

impl HeapMarkAndSweep for JsError {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.0.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.0.sweep_values(compactions);
    }
}

// #[derive(Debug)]
// pub struct PreAllocated;

pub(crate) enum InnerJob {
    PromiseResolveThenable(PromiseResolveThenableJob),
    PromiseReaction(PromiseReactionJob),
}

pub struct Job {
    pub(crate) realm: Option<RealmIdentifier<'static>>,
    pub(crate) inner: InnerJob,
}

impl Job {
    fn realm(&self) -> Option<RealmIdentifier<'static>> {
        self.realm
    }

    pub fn run(self, agent: &mut Agent, gc: GcScope) -> JsResult<()> {
        let mut pushed_context = false;
        if let Some(realm) = self.realm {
            if agent.current_realm(gc.nogc()) != realm {
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
            InnerJob::PromiseResolveThenable(job) => job.run(agent, gc),
            InnerJob::PromiseReaction(job) => job.run(agent, gc),
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

pub trait HostHooks: core::fmt::Debug {
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

    /// Get access to the Host data, useful to share state between calls of built-in functions.
    ///
    /// Note: This will panic if not implemented manually.
    fn get_host_data(&self) -> &dyn Any {
        unimplemented!()
    }
}

/// Owned ECMAScript Agent that can be used to run code but also to run garbage
/// collection on the Agent heap.
pub struct GcAgent {
    agent: Agent,
    realm_roots: Vec<Option<RealmIdentifier<'static>>>,
}

/// ECMAScript Realm root
///
/// As long as this is not passed back into GcAgent, the Realm it represents
/// won't be removed by the garbage collector.
#[must_use]
#[repr(transparent)]
pub struct RealmRoot {
    /// Defines an index in the GcAgent::realm_roots vector that contains the
    /// RealmIdentifier of this Realm.
    index: u8,
}

impl GcAgent {
    pub fn new(options: Options, host_hooks: &'static dyn HostHooks) -> Self {
        Self {
            agent: Agent::new(options, host_hooks),
            realm_roots: Vec::with_capacity(1),
        }
    }

    fn root_realm(&mut self, identifier: RealmIdentifier<'static>) -> RealmRoot {
        let index = if let Some((index, deleted_entry)) = self
            .realm_roots
            .iter_mut()
            .enumerate()
            .find(|(_, entry)| entry.is_none())
        {
            *deleted_entry = Some(identifier);
            index
        } else {
            self.realm_roots.push(Some(identifier));
            self.realm_roots.len() - 1
        };
        // Agent's Realm creation should've already popped the context that
        // created this Realm. The context stack should now be empty.
        assert!(self.agent.execution_context_stack.is_empty());
        RealmRoot {
            index: u8::try_from(index).expect("Only up to 256 simultaneous Realms are supported"),
        }
    }

    /// Creates a new Realm
    ///
    /// The Realm will not be removed by garbage collection until
    /// [`GcAgent::remove_realm`] is called.
    pub fn create_realm(
        &mut self,
        create_global_object: Option<
            impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        >,
        create_global_this_value: Option<
            impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        >,
        initialize_global_object: Option<impl FnOnce(&mut Agent, Object, GcScope)>,
    ) -> RealmRoot {
        let realm = self.agent.create_realm_internal(
            create_global_object,
            create_global_this_value,
            initialize_global_object,
        );
        self.root_realm(realm.unbind())
    }

    /// Creates a default realm suitable for basic testing only.
    pub fn create_default_realm(&mut self) -> RealmRoot {
        let realm = self.agent.create_default_realm().unbind();
        self.root_realm(realm)
    }

    /// Removes the given Realm. Resources associated with the Realm are free
    /// to be collected by the garbage collector after this call.
    pub fn remove_realm(&mut self, realm: RealmRoot) {
        let RealmRoot { index } = realm;
        let error_message = "Cannot remove a non-existing Realm";
        // After this removal, the Realm can be collected by GC.
        let _ = self
            .realm_roots
            .get_mut(index as usize)
            .expect(error_message)
            .take()
            .expect(error_message);
        while !self.realm_roots.is_empty() && self.realm_roots.last().unwrap().is_none() {
            let _ = self.realm_roots.pop();
        }
    }

    pub fn run_in_realm<F, R>(&mut self, realm: &RealmRoot, func: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(&'agent mut Agent, GcScope<'gc, 'scope>) -> R,
    {
        let index = realm.index;
        let error_message = "Attempted to run in non-existing Realm";
        let realm = *self
            .realm_roots
            .get(index as usize)
            .expect(error_message)
            .as_ref()
            .expect(error_message);
        assert!(self.agent.execution_context_stack.is_empty());
        let result = self.agent.run_in_realm(realm, func);
        assert!(self.agent.execution_context_stack.is_empty());
        assert!(self.agent.vm_stack.is_empty());
        self.agent.stack_refs.borrow_mut().clear();
        result
    }

    pub fn gc(&mut self) {
        if self.agent.options.disable_gc {
            // GC is disabled; no-op
            return;
        }
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut gc, &mut scope);
        heap_gc(&mut self.agent, &mut self.realm_roots, gc);
    }
}

/// ### [9.7 Agents](https://tc39.es/ecma262/#sec-agents)
#[derive(Debug)]
pub struct Agent {
    pub(crate) heap: Heap,
    pub(crate) options: Options,
    pub(crate) symbol_id: usize,
    pub(crate) global_symbol_registry: AHashMap<&'static str, Symbol<'static>>,
    pub(crate) host_hooks: &'static dyn HostHooks,
    pub(crate) execution_context_stack: Vec<ExecutionContext>,
    /// Temporary storage for on-stack heap roots.
    ///
    /// TODO: With Realm-specific heaps we'll need a side-table to define which
    /// Realm a particular stack value points to.
    pub(crate) stack_refs: RefCell<Vec<HeapRootData>>,
    /// Temporary storage for on-stack heap root collections.
    pub(crate) stack_ref_collections: RefCell<Vec<HeapRootCollectionData>>,
    /// Temporary storage for on-stack VMs.
    pub(crate) vm_stack: Vec<NonNull<Vm>>,
}

impl Agent {
    pub(crate) fn new(options: Options, host_hooks: &'static dyn HostHooks) -> Self {
        Self {
            heap: Heap::new(),
            options,
            symbol_id: 0,
            global_symbol_registry: AHashMap::default(),
            host_hooks,
            execution_context_stack: Vec::new(),
            stack_refs: RefCell::new(Vec::with_capacity(64)),
            stack_ref_collections: RefCell::new(Vec::with_capacity(32)),
            vm_stack: Vec::with_capacity(16),
        }
    }

    fn get_created_realm_root(&mut self) -> RealmIdentifier<'static> {
        assert!(!self.execution_context_stack.is_empty());
        let identifier = self.current_realm_id_internal();
        let _ = self.execution_context_stack.pop();
        identifier.unbind()
    }

    /// Creates a new Realm
    ///
    /// This is intended for usage within BuiltinFunction calls.
    pub fn create_realm<'gc>(
        &mut self,
        create_global_object: Option<
            impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        >,
        create_global_this_value: Option<
            impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        >,
        initialize_global_object: Option<impl FnOnce(&mut Agent, Object, GcScope)>,
        gc: GcScope<'gc, '_>,
    ) -> RealmIdentifier<'gc> {
        initialize_host_defined_realm(
            self,
            create_global_object,
            create_global_this_value,
            initialize_global_object,
            gc,
        );
        self.get_created_realm_root()
    }

    fn create_realm_internal(
        &mut self,
        create_global_object: Option<
            impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        >,
        create_global_this_value: Option<
            impl for<'a> FnOnce(&mut Agent, GcScope<'a, '_>) -> Object<'a>,
        >,
        initialize_global_object: Option<impl FnOnce(&mut Agent, Object, GcScope)>,
    ) -> RealmIdentifier<'static> {
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut gc, &mut scope);

        initialize_host_defined_realm(
            self,
            create_global_object,
            create_global_this_value,
            initialize_global_object,
            gc,
        );
        self.get_created_realm_root()
    }

    /// Creates a default realm suitable for basic testing only.
    ///
    /// This is intended for usage within BuiltinFunction calls.
    fn create_default_realm(&mut self) -> RealmIdentifier {
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut gc, &mut scope);

        initialize_default_realm(self, gc);
        self.get_created_realm_root()
    }

    pub fn run_in_realm<F, R>(&mut self, realm: RealmIdentifier, func: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(&'agent mut Agent, GcScope<'gc, 'scope>) -> R,
    {
        let execution_stack_depth_before_call = self.execution_context_stack.len();
        self.execution_context_stack.push(ExecutionContext {
            ecmascript_code: None,
            function: None,
            realm: realm.unbind(),
            script_or_module: None,
        });
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut gc, &mut scope);

        let result = func(self, gc);
        assert_eq!(
            self.execution_context_stack.len(),
            execution_stack_depth_before_call + 1
        );
        self.execution_context_stack.pop();
        result
    }

    /// Get current Realm's global environment.
    pub fn current_global_env<'a>(&self, gc: NoGcScope<'a, '_>) -> GlobalEnvironment<'a> {
        let realm = self.current_realm(gc);
        self[realm].global_env.unwrap().bind(gc)
    }

    /// Get the [current Realm](https://tc39.es/ecma262/#current-realm).
    pub fn current_realm<'a>(&self, gc: NoGcScope<'a, '_>) -> RealmIdentifier<'a> {
        self.current_realm_id_internal().bind(gc)
    }

    /// Set the current executiono context's Realm.
    pub(crate) fn set_current_realm(&mut self, realm: RealmIdentifier) {
        self.execution_context_stack.last_mut().unwrap().realm = realm.unbind();
    }

    /// Internal method to get current Realm's identifier without binding.
    pub(crate) fn current_realm_id_internal(&self) -> RealmIdentifier<'static> {
        self.execution_context_stack.last().unwrap().realm
    }

    pub(crate) fn current_realm_record(&self) -> &Realm {
        self.get_realm_record_by_id(self.current_realm_id_internal())
    }

    pub(crate) fn current_realm_record_mut(&mut self) -> &mut Realm<'static> {
        self.get_realm_record_by_id_mut(self.current_realm_id_internal())
    }

    pub(crate) fn get_realm_record_by_id(&self, id: RealmIdentifier) -> &Realm {
        &self[id]
    }

    fn get_realm_record_by_id_mut(&mut self, id: RealmIdentifier) -> &mut Realm<'static> {
        &mut self[id]
    }

    pub fn create_exception_with_static_message(
        &mut self,
        kind: ExceptionType,
        message: &'static str,
        gc: NoGcScope,
    ) -> Value {
        let message = String::from_static_str(self, message, gc).unbind();
        self.heap
            .create(ErrorHeapData::new(kind, Some(message), None))
            .into_value()
    }

    /// ### [5.2.3.2 Throw an Exception](https://tc39.es/ecma262/#sec-throw-an-exception)
    pub fn throw_exception_with_static_message(
        &mut self,
        kind: ExceptionType,
        message: &'static str,
        gc: NoGcScope,
    ) -> JsError {
        JsError(
            self.create_exception_with_static_message(kind, message, gc)
                .unbind(),
        )
    }

    pub fn throw_exception(
        &mut self,
        kind: ExceptionType,
        message: std::string::String,
        gc: NoGcScope,
    ) -> JsError {
        let message = String::from_string(self, message, gc).unbind();
        JsError(
            self.heap
                .create(ErrorHeapData::new(kind, Some(message), None))
                .into_value(),
        )
    }

    pub fn throw_exception_with_message(
        &mut self,
        kind: ExceptionType,
        message: String,
    ) -> JsError {
        JsError(
            self.heap
                .create(ErrorHeapData::new(kind, Some(message.unbind()), None))
                .into_value(),
        )
    }

    pub(crate) fn running_execution_context(&self) -> &ExecutionContext {
        self.execution_context_stack.last().unwrap()
    }

    pub(crate) fn current_source_code<'a>(&self, gc: NoGcScope<'a, '_>) -> SourceCode<'a> {
        self.execution_context_stack
            .last()
            .unwrap()
            .ecmascript_code
            .as_ref()
            .unwrap()
            .source_code
            .bind(gc)
    }

    /// Returns the running execution context's LexicalEnvironment.
    pub(crate) fn current_lexical_environment<'a>(&self, gc: NoGcScope<'a, '_>) -> Environment<'a> {
        self.execution_context_stack
            .last()
            .unwrap()
            .ecmascript_code
            .as_ref()
            .unwrap()
            .lexical_environment
            .bind(gc)
    }

    /// Returns the running execution context's VariableEnvironment.
    pub(crate) fn current_variable_environment<'a>(
        &self,
        gc: NoGcScope<'a, '_>,
    ) -> Environment<'a> {
        self.execution_context_stack
            .last()
            .unwrap()
            .ecmascript_code
            .as_ref()
            .unwrap()
            .variable_environment
            .bind(gc)
    }

    /// Returns the running execution context's PrivateEnvironment.
    pub(crate) fn current_private_environment<'a>(
        &self,
        gc: NoGcScope<'a, '_>,
    ) -> Option<PrivateEnvironment<'a>> {
        self.execution_context_stack
            .last()
            .unwrap()
            .ecmascript_code
            .as_ref()
            .unwrap()
            .private_environment
            .bind(gc)
    }

    /// Sets the running execution context's LexicalEnvironment.
    pub(crate) fn set_current_lexical_environment(&mut self, env: Environment) {
        self.execution_context_stack
            .last_mut()
            .unwrap()
            .ecmascript_code
            .as_mut()
            .unwrap()
            .lexical_environment = env.unbind();
    }

    /// Sets the running execution context's VariableEnvironment.
    pub(crate) fn set_current_variable_environment(&mut self, env: Environment) {
        self.execution_context_stack
            .last_mut()
            .unwrap()
            .ecmascript_code
            .as_mut()
            .unwrap()
            .variable_environment = env.unbind();
    }

    /// Sets the running execution context's PrivateEnvironment.
    pub(crate) fn set_current_private_environment(&mut self, env: PrivateEnvironment) {
        self.execution_context_stack
            .last_mut()
            .unwrap()
            .ecmascript_code
            .as_mut()
            .unwrap()
            .private_environment = Some(env.unbind());
    }

    pub(crate) fn running_execution_context_mut(&mut self) -> &mut ExecutionContext {
        self.execution_context_stack.last_mut().unwrap()
    }

    /// Panics if no active function object exists.
    pub(crate) fn active_function_object<'a>(&self, gc: NoGcScope<'a, '_>) -> Function<'a> {
        self.execution_context_stack
            .last()
            .unwrap()
            .function
            .unwrap()
            .bind(gc)
    }

    /// Get access to the Host data, useful to share state between calls of built-in functions.
    ///
    /// Note: This will panic if not implemented manually.
    pub fn get_host_data(&self) -> &dyn Any {
        self.host_hooks.get_host_data()
    }

    /// Run a script in the current Realm.
    pub fn run_script<'gc>(
        &mut self,
        source_text: String,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let realm = self.current_realm(gc.nogc());
        let script = match parse_script(self, source_text, realm, false, None, gc.nogc()) {
            Ok(script) => script,
            Err(err) => {
                let message =
                    String::from_string(self, err.first().unwrap().message.to_string(), gc.nogc());
                return Err(self.throw_exception_with_message(ExceptionType::SyntaxError, message));
            }
        };
        script_evaluation(self, script.unbind(), gc)
    }
}

/// ### [9.4.1 GetActiveScriptOrModule ()](https://tc39.es/ecma262/#sec-getactivescriptormodule)
///
/// The abstract operation GetActiveScriptOrModule takes no arguments and
/// returns a Script Record, a Module Record, or null. It is used to determine
/// the running script or module, based on the running execution context.
pub(crate) fn get_active_script_or_module<'a>(
    agent: &mut Agent,
    _: NoGcScope<'a, '_>,
) -> Option<ScriptOrModule<'a>> {
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

/// ### Try [9.4.2 ResolveBinding ( name \[ , env \] )](https://tc39.es/ecma262/#sec-resolvebinding)
///
/// The abstract operation ResolveBinding takes argument name (a String) and
/// optional argument env (an Environment Record or undefined) and returns
/// either a normal completion containing a Reference Record or a throw
/// completion. It is used to determine the binding of name. env can be used to
/// explicitly provide the Environment Record that is to be searched for the
/// binding.
pub(crate) fn try_resolve_binding<'a>(
    agent: &mut Agent,
    name: String<'a>,
    env: Option<Environment>,
    gc: NoGcScope<'a, '_>,
) -> TryResult<Reference<'a>> {
    let env = env
        .unwrap_or_else(|| {
            // 1. If env is not present or env is undefined, then
            //    a. Set env to the running execution context's LexicalEnvironment.
            agent.current_lexical_environment(gc)
        })
        .bind(gc);

    // 2. Assert: env is an Environment Record.
    // Implicit from env's type.

    // 3. Let strict be IsStrict(the syntactic production that is being evaluated).
    let strict = agent
        .running_execution_context()
        .ecmascript_code
        .unwrap()
        .is_strict_mode;

    // 4. Return ? GetIdentifierReference(env, name, strict).
    try_get_identifier_reference(agent, Some(env), name, strict, gc)
}

/// ### [9.4.2 ResolveBinding ( name \[ , env \] )](https://tc39.es/ecma262/#sec-resolvebinding)
///
/// The abstract operation ResolveBinding takes argument name (a String) and
/// optional argument env (an Environment Record or undefined) and returns
/// either a normal completion containing a Reference Record or a throw
/// completion. It is used to determine the binding of name. env can be used to
/// explicitly provide the Environment Record that is to be searched for the
/// binding.
pub(crate) fn resolve_binding<'a, 'b>(
    agent: &mut Agent,
    name: String<'b>,
    env: Option<Environment>,
    gc: GcScope<'a, 'b>,
) -> JsResult<Reference<'a>> {
    let name = name.bind(gc.nogc());
    let env = env
        .unwrap_or_else(|| {
            // 1. If env is not present or env is undefined, then
            //    a. Set env to the running execution context's LexicalEnvironment.
            agent.current_lexical_environment(gc.nogc())
        })
        .bind(gc.nogc());

    // 2. Assert: env is an Environment Record.
    // Implicit from env's type.

    // 3. Let strict be IsStrict(the syntactic production that is being evaluated).
    let strict = agent
        .running_execution_context()
        .ecmascript_code
        .unwrap()
        .is_strict_mode;

    // 4. Return ? GetIdentifierReference(env, name, strict).
    get_identifier_reference(agent, Some(env.unbind()), name.unbind(), strict, gc)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl TryFrom<u16> for ExceptionType {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, ()> {
        match value {
            0 => Ok(Self::Error),
            1 => Ok(Self::AggregateError),
            2 => Ok(Self::EvalError),
            3 => Ok(Self::RangeError),
            4 => Ok(Self::ReferenceError),
            5 => Ok(Self::SyntaxError),
            6 => Ok(Self::TypeError),
            7 => Ok(Self::UriError),
            _ => Err(()),
        }
    }
}

impl PrimitiveHeapIndexable for Agent {}
