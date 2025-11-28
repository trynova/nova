// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [9.6 Agents](https://tc39.es/ecma262/#sec-agents)
//!
//! An _agent_ comprises a set of ECMAScript
//!# [execution contexts](https://tc39.es/ecma262/#sec-execution-contexts), an
//! execution context stack, a running execution context, an _Agent Record_,
//! and an _executing thread_. Except for the
//!# [executing thread](https://tc39.es/ecma262/#executing-thread), the
//! constituents of an agent belong exclusively to that agent.
//!
//! In Nova, the [`Agent Record`](Agent) is the main entry point into the
//! JavaScript virtual machine and its heap memory.
//!
//! ### Notes
//!
//! - This is inspired by and/or copied from Kiesel engine:
//!   Copyright (c) 2023-2024 Linus Groh

use ahash::AHashMap;

#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "atomics")]
use crate::ecmascript::builtins::structured_data::atomics_object::WaitAsyncJob;
#[cfg(feature = "weak-refs")]
use crate::ecmascript::execution::{FinalizationRegistryCleanupJob, clear_kept_objects};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_string,
        builtins::{
            error::ErrorHeapData,
            ordinary::caches::PropertyLookupCache,
            promise::Promise,
            promise_objects::promise_abstract_operations::promise_jobs::{
                PromiseReactionJob, PromiseResolveThenableJob,
            },
        },
        scripts_and_modules::{
            ScriptOrModule,
            module::module_semantics::{
                ModuleRequest, Referrer, abstract_module_records::AbstractModuleMethods,
                cyclic_module_records::GraphLoadingStateRecord,
                source_text_module_records::SourceTextModule,
            },
            script::{HostDefined, parse_script, script_evaluation},
            source_code::SourceCode,
        },
        types::{
            Function, IntoValue, Object, OrdinaryObject, PrivateName, PropertyKey, Reference,
            String, Symbol, Value, ValueRootRepr,
        },
    },
    engine::{
        Vm,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootCollectionData, HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, PrimitiveHeapIndexable,
        WorkQueues, heap_gc::heap_gc,
    },
    ndt,
};

use super::{
    Environment, ExecutionContext, GlobalEnvironment, PrivateEnvironment, Realm, RealmRecord,
    environments::{get_identifier_reference, try_get_identifier_reference},
    initialize_default_realm, initialize_host_defined_realm,
};
use core::{any::Any, cell::RefCell, ops::ControlFlow, ptr::NonNull};
use std::{collections::TryReserveError, sync::Arc};

#[derive(Debug, Default)]
pub struct Options {
    pub disable_gc: bool,
    pub print_internals: bool,
    /// Controls the \[\[CanBlock]] option of the Agent Record. If set to true,
    /// calling `Atomics.wait()` will throw an error to signal that blocking
    /// the main thread is not allowed.
    pub no_block: bool,
}

pub type JsResult<'a, T> = core::result::Result<T, JsError<'a>>;

impl<'a, T: 'a> From<JsError<'a>> for JsResult<'a, T> {
    fn from(value: JsError<'a>) -> Self {
        JsResult::Err(value)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct JsError<'a>(Value<'a>);
bindable_handle!(JsError);

impl<'a> JsError<'a> {
    pub(crate) fn new(value: Value<'a>) -> Self {
        Self(value)
    }

    pub fn value(self) -> Value<'a> {
        self.0
    }

    pub fn to_string<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> String<'gc> {
        to_string(agent, self.0, gc).unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct JsErrorRootRepr(ValueRootRepr);

impl Rootable for JsError<'_> {
    type RootRepr = JsErrorRootRepr;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Value::to_root_repr(value.value()).map(JsErrorRootRepr)
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Value::from_root_repr(&value.0).map(JsError)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        JsErrorRootRepr(Value::from_heap_ref(heap_ref))
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        Value::from_heap_data(heap_data).map(JsError)
    }
}

impl HeapMarkAndSweep for JsError<'static> {
    fn mark_values(&self, queues: &mut crate::heap::WorkQueues) {
        self.0.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &crate::heap::CompactionLists) {
        self.0.sweep_values(compactions);
    }
}

/// Failure conditions for internal method's Try variants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TryError<'a> {
    /// The method threw an error.
    Err(JsError<'a>),
    /// The method cannot run to completion without calling into JavaScript.
    ///
    /// > Note 1: methods can and are encouraged to delegate any JavaScript
    /// > tail calls to the caller (such as getter, setter, or Proxy trap call
    /// > at the end of a \[\[Get]] or \[\[Set]] method). This variant should
    /// > be used when the method would need to perform additional work after
    /// > the JavaScript call is done.
    ///
    /// > Note 2: Returning this error indicates that the entire operation will
    /// > be rerun from start to finish in a GC-capable scope. The Try method
    /// > variant must therefore be undetectable; it cannot perform mutations
    /// > that would affect how the normal variant runs.
    GcError,
}
bindable_handle!(TryError);

pub fn option_into_try<'a, T: 'a>(value: Option<T>) -> TryResult<'a, T> {
    match value {
        Some(value) => TryResult::Continue(value),
        None => TryError::GcError.into(),
    }
}

/// Convert a JsResult into a TryResult.
///
/// This is useful when an abstract operation can throw errors but cannot call
/// into JavaScript, and is called from a Try method. The AO returns a JsResult
/// but the caller wants to convert it into a TryResult before returning.
pub fn js_result_into_try<'a, T: 'a>(value: JsResult<'a, T>) -> TryResult<'a, T> {
    match value {
        Ok(value) => TryResult::Continue(value),
        Err(err) => TryResult::Break(TryError::Err(err)),
    }
}

/// Convert a `TryResult<T>` into a [JsResult] of an `Option<T>`.
///
/// This is useful when a method that may trigger GC calls into a Try method
/// and wants to rethrow any errors and use the result if available.
pub fn try_result_into_js<'a, T: 'a>(value: TryResult<'a, T>) -> JsResult<'a, Option<T>> {
    match value {
        TryResult::Continue(value) => JsResult::Ok(Some(value)),
        TryResult::Break(TryError::GcError) => JsResult::Ok(None),
        TryResult::Break(TryError::Err(err)) => JsResult::Err(err),
    }
}

/// Convert a `TryResult<T>` into an `Option<JsResult<T>>`.
///
/// This is useful when a method that may trigger GC calls into a Try method
/// and wants to use the result if available, error or not.
pub fn try_result_into_option_js<'a, T: 'a>(value: TryResult<'a, T>) -> Option<JsResult<'a, T>> {
    match value {
        TryResult::Continue(value) => Some(JsResult::Ok(value)),
        TryResult::Break(TryError::GcError) => None,
        TryResult::Break(TryError::Err(err)) => Some(JsResult::Err(err)),
    }
}

impl<'a, T: 'a> From<JsError<'a>> for TryResult<'a, T> {
    fn from(value: JsError<'a>) -> Self {
        TryResult::Break(TryError::Err(value))
    }
}

impl<'a, T: 'a> From<TryError<'a>> for TryResult<'a, T> {
    fn from(value: TryError<'a>) -> Self {
        TryResult::Break(value)
    }
}

macro_rules! try_result_ok {
    ($self:ident) => {
        impl<'a> core::convert::From<$self<'a>> for TryResult<'a, $self<'a>> {
            fn from(value: $self<'a>) -> Self {
                TryResult::Continue(value)
            }
        }
    };
}
pub(crate) use try_result_ok;

/// Result of methods that are not allowed to call JavaScript or perform
/// garbage collection.
pub type TryResult<'a, T> = ControlFlow<TryError<'a>, T>;

#[inline]
pub fn unwrap_try<'a, T: 'a>(try_result: TryResult<'a, T>) -> T {
    match try_result {
        TryResult::Continue(t) => t,
        TryResult::Break(_) => unreachable!(),
    }
}

pub(crate) enum InnerJob {
    PromiseResolveThenable(PromiseResolveThenableJob),
    PromiseReaction(PromiseReactionJob),
    #[cfg(feature = "atomics")]
    WaitAsync(WaitAsyncJob),
    #[cfg(feature = "weak-refs")]
    FinalizationRegistry(FinalizationRegistryCleanupJob),
}

pub struct Job {
    pub(crate) realm: Option<Realm<'static>>,
    pub(crate) inner: InnerJob,
}

impl Job {
    pub fn is_finished(&self) -> bool {
        match &self.inner {
            #[cfg(feature = "atomics")]
            InnerJob::WaitAsync(job) => job.is_finished(),
            _ => true,
        }
    }

    pub fn run<'a>(self, agent: &mut Agent, gc: GcScope<'a, '_>) -> JsResult<'a, ()> {
        let mut id = 0;
        ndt::job_evaluation_start!(|| {
            id = core::ptr::from_ref(&self).addr() as u64;
            id
        });
        let mut pushed_context = false;
        if let Some(realm) = self.realm
            && agent.current_realm(gc.nogc()) != realm
        {
            agent.push_execution_context(ExecutionContext {
                ecmascript_code: None,
                function: None,
                realm,
                script_or_module: None,
            });
            pushed_context = true;
        }

        let result = match self.inner {
            InnerJob::PromiseResolveThenable(job) => job.run(agent, gc),
            InnerJob::PromiseReaction(job) => job.run(agent, gc),
            #[cfg(feature = "atomics")]
            InnerJob::WaitAsync(job) => job.run(agent, gc),
            #[cfg(feature = "weak-refs")]
            InnerJob::FinalizationRegistry(job) => {
                job.run(agent, gc);
                Ok(())
            }
        };

        if pushed_context {
            agent.execution_context_stack.pop();
        }

        ndt::job_evaluation_done!(|| id);

        result
    }
}

pub enum PromiseRejectionTrackerOperation {
    Reject,
    Handle,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
#[cfg(feature = "shared-array-buffer")]
pub enum GrowSharedArrayBufferResult {
    #[default]
    Unhandled = 0,
    Handled = 1,
}

pub trait HostEnqueueGenericJobHandle {
    /// ### [9.5.4 HostEnqueueGenericJob ( job, realm )](https://tc39.es/ecma262/#sec-hostenqueuegenericjob)
    ///
    /// The host-defined abstract operation HostEnqueueGenericJob takes
    /// arguments _job_ (a Job Abstract Closure) and _realm_ (a Realm Record)
    /// and returns unused. It schedules _job_ in the realm realm in the agent
    /// signified by _realm_.\[\[AgentSignifier]] to be performed at some
    /// future time. The Abstract Closures used with this algorithm are
    /// intended to be scheduled without additional constraints, such as
    /// priority and ordering.
    ///
    /// An implementation of HostEnqueueGenericJob must conform to the
    /// requirements in 9.5.
    fn enqueue_generic_job(&self, job: Job);
}

pub trait HostHooks: core::fmt::Debug {
    /// ### [19.2.1.2 HostEnsureCanCompileStrings ( calleeRealm )](https://tc39.es/ecma262/#sec-hostensurecancompilestrings)
    #[allow(unused_variables)]
    fn ensure_can_compile_strings<'a>(
        &self,
        callee_realm: &mut RealmRecord,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        // The default implementation of HostEnsureCanCompileStrings is to return NormalCompletion(unused).
        Ok(())
    }

    /// ### [20.2.5 HostHasSourceTextAvailable ( func )](https://tc39.es/ecma262/#sec-hosthassourcetextavailable)
    #[allow(unused_variables)]
    fn has_source_text_available(&self, func: Function) -> bool {
        // The default implementation of HostHasSourceTextAvailable is to return true.
        true
    }

    /// Get a shareable handle to the HostEnqueueGenericJob function.
    ///
    /// This is used by off-thread tasks that may or may not ever resolve. By
    /// default the function returns None to indicate that off-thread tasks are
    /// not supported.
    fn get_enqueue_generic_job_handle(&self) -> Option<Arc<dyn HostEnqueueGenericJobHandle>> {
        None
    }

    /// ### [9.5.4 HostEnqueueGenericJob ( job, realm )](https://tc39.es/ecma262/#sec-hostenqueuegenericjob)
    ///
    /// The host-defined abstract operation HostEnqueueGenericJob takes
    /// arguments _job_ (a Job Abstract Closure) and _realm_ (a Realm Record)
    /// and returns unused. It schedules _job_ in the realm realm in the agent
    /// signified by _realm_.\[\[AgentSignifier]] to be performed at some
    /// future time. The Abstract Closures used with this algorithm are
    /// intended to be scheduled without additional constraints, such as
    /// priority and ordering.
    ///
    /// An implementation of HostEnqueueGenericJob must conform to the
    /// requirements in 9.5.
    fn enqueue_generic_job(&self, job: Job);

    /// ### [9.5.5 HostEnqueuePromiseJob ( job, realm )](https://tc39.es/ecma262/#sec-hostenqueuepromisejob)
    fn enqueue_promise_job(&self, job: Job);

    /// ### [9.5.6 HostEnqueueTimeoutJob ( timeoutJob, realm, milliseconds )](https://tc39.es/ecma262/#sec-hostenqueuetimeoutjob)
    ///
    /// The host-defined abstract operation HostEnqueueTimeoutJob takes
    /// arguments _timeoutJob_ (a Job Abstract Closure), _realm_ (a Realm
    /// Record), and _milliseconds_ (a non-negative finite Number) and returns
    /// unused. It schedules _timeoutJob_ in the realm _realm_ in the agent
    /// signified by _realm_.\[\[AgentSignifier]] to be performed after at
    /// least _milliseconds_ milliseconds.
    ///
    /// An implementation of HostEnqueueTimeoutJob must conform to the
    /// requirements in 9.5.
    fn enqueue_timeout_job(&self, timeout_job: Job, milliseconds: u64);

    /// ### [9.9.4.1 HostEnqueueFinalizationRegistryCleanupJob ( finalizationRegistry )](https://tc39.es/ecma262/#sec-host-cleanup-finalization-registry)
    ///
    /// The host-defined abstract operation
    /// HostEnqueueFinalizationRegistryCleanupJob takes argument
    /// _finalizationRegistry_ (a FinalizationRegistry) and returns unused.
    ///
    /// Let _cleanupJob_ be a new Job Abstract Closure with no parameters that
    /// captures _finalizationRegistry_ and performs the following steps when
    /// called:
    ///
    /// ```text
    /// 1. Let cleanupResult be
    ///    Completion(CleanupFinalizationRegistry(finalizationRegistry)).
    /// 2. If cleanupResult is an abrupt completion, perform any host-defined
    ///    steps for reporting the error.
    /// 3. Return unused.
    /// ```
    ///
    /// An implementation of HostEnqueueFinalizationRegistryCleanupJob
    /// schedules cleanupJob to be performed at some future time, if possible.
    /// It must also conform to the requirements in 9.5.
    #[allow(unused_variables)]
    #[cfg(feature = "weak-refs")]
    fn enqueue_finalization_registry_cleanup_job(&self, job: Job) {
        // By default, just ignore cleanup.
    }

    /// ### [27.2.1.9 HostPromiseRejectionTracker ( promise, operation )](https://tc39.es/ecma262/#sec-host-promise-rejection-tracker)
    #[allow(unused_variables)]
    fn promise_rejection_tracker(
        &self,
        promise: Promise,
        operation: PromiseRejectionTrackerOperation,
    ) {
        // The default implementation of HostPromiseRejectionTracker is to return unused.
    }

    /// ### [16.2.1.10 HostLoadImportedModule ( referrer, moduleRequest, hostDefined, payload )](https://tc39.es/ecma262/#sec-HostLoadImportedModule)
    ///
    /// The host-defined abstract operation HostLoadImportedModule takes
    /// arguments referrer (a Script Record, a Cyclic Module Record, or a Realm
    /// Record), moduleRequest (a ModuleRequest Record), hostDefined
    /// (anything), and payload (a GraphLoadingState Record or a
    /// PromiseCapability Record) and returns unused.
    ///
    /// > NOTE 1: An example of when referrer can be a Realm Record is in a web
    /// > browser host. There, if a user clicks on a control given by
    /// > ```html
    /// > <button type="button" onclick="import('./foo.mjs')">Click me</button>
    /// > ```
    /// > there will be no active script or module at the time the `import()`
    /// > expression runs. More generally, this can happen in any situation
    /// > where the host pushes execution contexts with null ScriptOrModule
    /// > components onto the execution context stack.
    ///
    /// An implementation of HostLoadImportedModule must conform to the
    /// following requirements:
    /// * The host environment must perform `FinishLoadingImportedModule
    ///   referrer, moduleRequest, payload, result)`, where `result` is either
    ///   a normal completion containing the loaded Module Record or a throw
    ///   completion, either synchronously or asynchronously.
    ///
    /// * If this operation is called multiple times with two `(referrer,
    ///   moduleRequest)` pairs such that:
    ///
    ///   * the first `referrer` is the same as the second `referrer`;
    ///
    ///   * `ModuleRequestsEqual(the first moduleRequest, the second
    ///     moduleRequest)` is true;
    ///
    ///   and it performs `FinishLoadingImportedModule(referrer, moduleRequest,
    ///   payload, result)` where `result` is a normal completion, then it must
    ///   perform `FinishLoadingImportedModule(referrer, moduleRequest,
    ///   payload, result)` with the same result each time.
    ///
    /// * If `moduleRequest.[[Attributes]]` has an entry entry such that
    ///   `entry.[[Key]]` is "type" and `entry.[[Value]]` is "json", when the
    ///   host environment performs `FinishLoadingImportedModule(referrer,
    ///   moduleRequest, payload, result)`, result must either be the
    ///   Completion Record returned by an invocation of `ParseJSONModule` or a
    ///   throw completion.
    ///
    /// * The operation must treat `payload` as an opaque value to be passed
    ///   through to `FinishLoadingImportedModule`.
    ///
    /// The actual process performed is host-defined, but typically consists of
    /// performing whatever I/O operations are necessary to load the
    /// appropriate Module Record. Multiple different `(referrer,
    /// moduleRequest.[[Specifier]], moduleRequest.[[Attributes]])` triples may
    /// map to the same Module Record instance. The actual mapping semantics is
    /// host-defined but typically a normalization process is applied to
    /// specifier as part of the mapping process. A typical normalization
    /// process would include actions such as expansion of relative and
    /// abbreviated path specifiers.
    ///
    /// > NOTE 2: The above text requires that hosts support JSON modules when
    /// > imported with `type: "json"` (and `HostLoadImportedModule` completes
    /// > normally), but it does not prohibit hosts from supporting JSON
    /// > modules when imported without `type: "json"`.
    #[allow(unused_variables)]
    fn load_imported_module<'gc>(
        &self,
        agent: &mut Agent,
        referrer: Referrer<'gc>,
        module_request: ModuleRequest<'gc>,
        host_defined: Option<HostDefined>,
        payload: &mut GraphLoadingStateRecord<'gc>,
        gc: NoGcScope<'gc, '_>,
    ) {
        unimplemented!();
    }

    /// ### [13.3.12.1.1 HostGetImportMetaProperties ( moduleRecord )](https://tc39.es/ecma262/#sec-hostgetimportmetaproperties)
    ///
    /// The host-defined abstract operation HostGetImportMetaProperties takes
    /// argument moduleRecord (a Module Record) and returns a List of Records
    /// with fields \[\[Key]] (a property key) and \[\[Value]] (an ECMAScript
    /// language value). It allows hosts to provide property keys and values
    /// for the object returned from `import.meta`.
    ///
    /// The default implementation of HostGetImportMetaProperties is to return
    /// a new empty List.
    #[allow(unused_variables)]
    fn get_import_meta_properties<'gc>(
        &self,
        agent: &mut Agent,
        module_record: SourceTextModule,
        gc: NoGcScope<'gc, '_>,
    ) -> Vec<(PropertyKey<'gc>, Value<'gc>)> {
        Default::default()
    }

    /// ### [13.3.12.1.2 HostFinalizeImportMeta ( importMeta, moduleRecord )](https://tc39.es/ecma262/#sec-hostfinalizeimportmeta)
    ///
    /// The host-defined abstract operation HostFinalizeImportMeta takes
    /// arguments importMeta (an Object) and moduleRecord (a Module Record) and
    /// returns unused. It allows hosts to perform any extraordinary operations
    /// to prepare the object returned from import.meta.
    ///
    /// Most hosts will be able to simply define HostGetImportMetaProperties,
    /// and leave HostFinalizeImportMeta with its default behaviour. However,
    /// HostFinalizeImportMeta provides an "escape hatch" for hosts which need
    /// to directly manipulate the object before it is exposed to ECMAScript
    /// code.
    ///
    /// The default implementation of HostFinalizeImportMeta is to return
    /// unused.
    #[allow(unused_variables)]
    fn finalize_import_meta(
        &self,
        agent: &mut Agent,
        import_meta: OrdinaryObject,
        module_record: SourceTextModule,
        gc: NoGcScope,
    ) {
    }

    /// ### [25.2.2.3 HostGrowSharedArrayBuffer ( buffer, newByteLength )](tc39.es/ecma262/#sec-hostgrowsharedarraybuffer)
    ///
    /// The host-defined abstract operation HostGrowSharedArrayBuffer takes
    /// arguments `buffer` (a SharedArrayBuffer) and `newByteLength` (a
    /// non-negative integer) and returns either a normal completion containing
    /// either HANDLED or UNHANDLED, or a throw completion. It gives the host
    /// an opportunity to perform implementation-defined growing of `buffer`.
    /// If the host chooses not to handle growing of `buffer`, it may return
    /// UNHANDLED for the default behaviour.
    ///
    /// The implementation of HostGrowSharedArrayBuffer must conform to the
    /// following requirements:
    ///
    /// * If the abstract operation does not complete normally with UNHANDLED,
    ///   and `newByteLength` < the current byte length of the `buffer` or
    ///   `newByteLength` > `buffer.[[ArrayBufferMaxByteLength]]`, throw a
    ///   RangeError exception.
    /// * Let `isLittleEndian` be the value of the `[[LittleEndian]]` field of
    ///   the surrounding agent's Agent Record. If the abstract operation
    ///   completes normally with HANDLED, a WriteSharedMemory or
    ///   ReadModifyWriteSharedMemory event whose `[[Order]]` is seq-cst,
    ///   `[[Payload]]` is `NumericToRawBytes(biguint64, newByteLength, isLittleEndian)`,
    ///   `[[Block]]` is `buffer.[[ArrayBufferByteLengthData]]`, `[[ByteIndex]]`
    ///   is 0, and `[[ElementSize]]` is 8 is added to the surrounding agent's
    ///   candidate execution such that racing calls to
    ///   `SharedArrayBuffer.prototype.grow` are not "lost", i.e. silently do
    ///   nothing.
    ///
    /// > NOTE: The second requirement above is intentionally vague about how
    /// > or when the current byte length of buffer is read. Because the byte
    /// > length must be updated via an atomic read-modify-write operation on
    /// > the underlying hardware, architectures that use
    /// > load-link/store-conditional or load-exclusive/store-exclusive
    /// > instruction pairs may wish to keep the paired instructions close in
    /// > the instruction stream. As such, `SharedArrayBuffer.prototype.grow`
    /// > itself does not perform bounds checking on newByteLength before
    /// > calling HostGrowSharedArrayBuffer, nor is there a requirement on when
    /// > the current byte length is read.
    /// >
    /// > This is in contrast with HostResizeArrayBuffer, which is guaranteed
    /// > that the value of `newByteLength` is `≥ 0` and
    /// > `≤ buffer.[[ArrayBufferMaxByteLength]]`.
    #[allow(unused_variables)]
    #[inline(always)]
    #[cfg(feature = "shared-array-buffer")]
    fn grow_shared_array_buffer<'gc>(
        &self,
        agent: &Agent,
        buffer: SharedArrayBuffer,
        new_byte_length: u64,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, GrowSharedArrayBufferResult> {
        Ok(GrowSharedArrayBufferResult::Unhandled)
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
    realm_roots: Vec<Option<Realm<'static>>>,
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

impl RealmRoot {
    /// Initialize the Realm's \[\[HostDefined]] field to a value.
    ///
    /// ## Panics
    ///
    /// Panics if the \[\[HostDefined]] field is non-empty.
    pub fn initialize_host_defined(&self, agent: &mut GcAgent, host_defined: HostDefined) {
        let realm = agent.get_realm_by_root(self);
        realm.initialize_host_defined(&mut agent.agent, host_defined);
    }
}

impl GcAgent {
    pub fn new(options: Options, host_hooks: &'static dyn HostHooks) -> Self {
        Self {
            agent: Agent::new(options, host_hooks),
            realm_roots: Vec::with_capacity(1),
        }
    }

    fn root_realm(&mut self, identifier: Realm<'static>) -> RealmRoot {
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
        let realm = self.agent.create_default_realm();
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
        while let Some(r) = self.realm_roots.last()
            && r.is_none()
        {
            let _ = self.realm_roots.pop();
        }
    }

    pub fn run_in_realm<F, R>(&mut self, realm: &RealmRoot, func: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(&'agent mut Agent, GcScope<'gc, 'scope>) -> R,
    {
        let realm = self.get_realm_by_root(realm);
        assert!(self.agent.execution_context_stack.is_empty());
        let result = self.agent.run_in_realm(realm, func);
        #[cfg(feature = "weak-refs")]
        clear_kept_objects(&mut self.agent);
        assert!(self.agent.execution_context_stack.is_empty());
        assert!(self.agent.vm_stack.is_empty());
        self.agent.stack_refs.borrow_mut().clear();
        result
    }

    pub fn run_job<F, R>(&mut self, job: Job, then: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(
            &'agent mut Agent,
            JsResult<'_, ()>,
            GcScope<'gc, 'scope>,
        ) -> R,
    {
        assert!(self.agent.execution_context_stack.is_empty());
        let result = self.agent.run_job(job, then);
        #[cfg(feature = "weak-refs")]
        clear_kept_objects(&mut self.agent);
        assert!(self.agent.execution_context_stack.is_empty());
        assert!(self.agent.vm_stack.is_empty());
        self.agent.stack_refs.borrow_mut().clear();
        result
    }

    fn get_realm_by_root(&self, realm_root: &RealmRoot) -> Realm<'static> {
        let index = realm_root.index;
        let error_message = "Couldn't find Realm by RealmRoot";
        *self
            .realm_roots
            .get(index as usize)
            .expect(error_message)
            .as_ref()
            .expect(error_message)
    }

    pub fn gc(&mut self) {
        if self.agent.options.disable_gc {
            // GC is disabled; no-op
            return;
        }
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut gc, &mut scope);
        let Self {
            agent, realm_roots, ..
        } = self;
        heap_gc(agent, realm_roots, gc);
    }
}

/// ## [9.7 Agents](https://tc39.es/ecma262/#sec-agents)
pub struct Agent {
    pub(crate) heap: Heap,
    pub(crate) options: Options,
    #[expect(dead_code)]
    symbol_id: usize,
    pub(crate) global_symbol_registry: AHashMap<String<'static>, Symbol<'static>>,
    pub(crate) host_hooks: &'static dyn HostHooks,
    execution_context_stack: Vec<ExecutionContext>,
    /// Temporary storage for on-stack heap roots.
    ///
    /// TODO: With Realm-specific heaps we'll need a side-table to define which
    /// Realm a particular stack value points to.
    pub(crate) stack_refs: RefCell<Vec<HeapRootData>>,
    /// Temporary storage for on-stack heap root collections.
    pub(crate) stack_ref_collections: RefCell<Vec<HeapRootCollectionData>>,
    /// Temporary storage for on-stack VMs.
    pub(crate) vm_stack: Vec<NonNull<Vm>>,
    /// ### \[\[KeptAlive]]
    ///
    /// > Note: instead of storing objects in a list here, we only store a
    /// > boolean to clear weak references as needed.
    #[cfg(feature = "weak-refs")]
    pub(super) kept_alive: bool,
    /// Global counter for PrivateNames. This only ever grows.
    private_names_counter: u32,
    /// ### \[\[ModuleAsyncEvaluationCount]]
    ///
    /// Initially 0, used to assign unique incrementing values to the
    /// \[\[AsyncEvaluationOrder]] field of modules that are asynchronous or
    /// have asynchronous dependencies.
    module_async_evaluation_count: u32,
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
            #[cfg(feature = "weak-refs")]
            kept_alive: false,
            private_names_counter: 0,
            module_async_evaluation_count: 0,
        }
    }

    /// Returns the value of the Agent's `[[CanBlock]]` field.
    pub fn can_suspend(&self) -> bool {
        !self.options.no_block
    }

    pub fn gc(&mut self, gc: GcScope) {
        let mut root_realms = self
            .heap
            .realms
            .iter()
            .enumerate()
            .map(|(i, _)| Some(Realm::from_index(i)))
            .collect::<Vec<_>>();
        heap_gc(self, &mut root_realms, gc);
    }

    /// Checks if garbage collection should be performed based on the number of
    /// bytes allocated since last garbage collection.
    pub(crate) fn check_gc(&mut self) -> bool {
        // Perform garbage collection if over 2 MiB of allocations have been
        // performed since last GC.
        const ALLOC_COUNTER_LIMIT: usize = 1024 * 1024 * 2;
        self.heap.alloc_counter > ALLOC_COUNTER_LIMIT
    }

    fn get_created_realm_root(&mut self) -> Realm<'static> {
        assert!(!self.execution_context_stack.is_empty());
        let identifier = self.current_realm_id_internal();
        let _ = self.pop_execution_context();
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
    ) -> Realm<'gc> {
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
    ) -> Realm<'static> {
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
    fn create_default_realm(&mut self) -> Realm<'static> {
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let gc = GcScope::new(&mut gc, &mut scope);

        initialize_default_realm(self, gc);
        self.get_created_realm_root()
    }

    fn run_in_realm<F, R>(&mut self, realm: Realm, func: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(&'agent mut Agent, GcScope<'gc, 'scope>) -> R,
    {
        let execution_stack_depth_before_call = self.execution_context_stack.len();
        self.push_execution_context(ExecutionContext {
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
        self.pop_execution_context();
        result
    }

    fn run_job<F, R>(&mut self, job: Job, then: F) -> R
    where
        F: for<'agent, 'gc, 'scope> FnOnce(
            &'agent mut Agent,
            JsResult<'_, ()>,
            GcScope<'gc, 'scope>,
        ) -> R,
    {
        let realm = job.realm.unwrap();
        let execution_stack_depth_before_call = self.execution_context_stack.len();
        self.push_execution_context(ExecutionContext {
            ecmascript_code: None,
            function: None,
            realm,
            script_or_module: None,
        });
        let (mut gc, mut scope) = unsafe { GcScope::create_root() };
        let mut gc = GcScope::new(&mut gc, &mut scope);

        let result = job.run(self, gc.reborrow()).unbind().bind(gc.nogc());
        let result = then(self, result.unbind(), gc);
        assert_eq!(
            self.execution_context_stack.len(),
            execution_stack_depth_before_call + 1
        );
        self.pop_execution_context();
        result
    }

    /// Get current Realm's global environment.
    pub fn current_global_env<'a>(&self, gc: NoGcScope<'a, '_>) -> GlobalEnvironment<'a> {
        let realm = self.current_realm(gc);
        let Some(e) = self[realm].global_env.bind(gc) else {
            panic_corrupted_agent()
        };
        e
    }

    /// Get current Realm's global object.
    pub fn current_global_object<'a>(&self, gc: NoGcScope<'a, '_>) -> Object<'a> {
        let realm = self.current_realm(gc);
        self[realm].global_object.bind(gc)
    }

    /// Get the [current Realm](https://tc39.es/ecma262/#current-realm).
    pub fn current_realm<'a>(&self, gc: NoGcScope<'a, '_>) -> Realm<'a> {
        self.current_realm_id_internal().bind(gc)
    }

    /// Set the current executiono context's Realm.
    pub(crate) fn set_current_realm(&mut self, realm: Realm) {
        let Some(ctx) = self.execution_context_stack.last_mut() else {
            panic_corrupted_agent()
        };
        ctx.realm = realm.unbind();
    }

    /// Internal method to get current Realm's identifier without binding.
    #[inline]
    pub(crate) fn current_realm_id_internal(&self) -> Realm<'static> {
        let Some(r) = self.execution_context_stack.last().map(|ctx| ctx.realm) else {
            panic_corrupted_agent()
        };
        r
    }

    pub(crate) fn current_realm_record(&self) -> &RealmRecord<'static> {
        self.get_realm_record_by_id(self.current_realm_id_internal())
    }

    pub(crate) fn current_realm_record_mut(&mut self) -> &mut RealmRecord<'static> {
        self.get_realm_record_by_id_mut(self.current_realm_id_internal())
    }

    pub(crate) fn get_realm_record_by_id<'r>(&self, id: Realm<'r>) -> &RealmRecord<'r> {
        &self[id]
    }

    fn get_realm_record_by_id_mut(&mut self, id: Realm) -> &mut RealmRecord<'static> {
        &mut self[id]
    }

    #[must_use]
    pub fn create_exception_with_static_message<'a>(
        &mut self,
        kind: ExceptionType,
        message: &'static str,
        gc: NoGcScope<'a, '_>,
    ) -> Value<'a> {
        let message = String::from_static_str(self, message, gc).unbind();
        self.heap
            .create(ErrorHeapData::new(kind, Some(message), None))
            .into_value()
    }

    #[must_use]
    pub(crate) fn todo<'a>(&mut self, feature: &'static str, gc: NoGcScope<'a, '_>) -> JsError<'a> {
        self.throw_exception(
            ExceptionType::Error,
            format!("{feature} not implemented"),
            gc,
        )
    }

    /// ### [5.2.3.2 Throw an Exception](https://tc39.es/ecma262/#sec-throw-an-exception)
    #[must_use]
    pub fn throw_exception_with_static_message<'a>(
        &mut self,
        kind: ExceptionType,
        message: &'static str,
        gc: NoGcScope<'a, '_>,
    ) -> JsError<'a> {
        JsError(
            self.create_exception_with_static_message(kind, message, gc)
                .unbind(),
        )
    }

    #[must_use]
    pub fn throw_exception<'a>(
        &mut self,
        kind: ExceptionType,
        message: std::string::String,
        gc: NoGcScope<'a, '_>,
    ) -> JsError<'a> {
        let message = String::from_string(self, message, gc).unbind();
        JsError(
            self.heap
                .create(ErrorHeapData::new(kind, Some(message), None))
                .into_value(),
        )
    }

    #[must_use]
    pub fn throw_exception_with_message<'a>(
        &mut self,
        kind: ExceptionType,
        message: String,
        gc: NoGcScope<'a, '_>,
    ) -> JsError<'a> {
        JsError(
            self.heap
                .create(ErrorHeapData::new(kind, Some(message.unbind()), None))
                .into_value()
                .bind(gc),
        )
    }

    #[must_use]
    pub(crate) fn throw_allocation_exception<'a>(
        &mut self,
        error: TryReserveError,
        gc: NoGcScope<'a, '_>,
    ) -> JsError<'a> {
        self.throw_exception(ExceptionType::RangeError, error.to_string(), gc)
    }

    pub(crate) fn running_execution_context(&self) -> &ExecutionContext {
        let Some(ctx) = self.execution_context_stack.last() else {
            panic_corrupted_agent()
        };
        ctx
    }

    pub(crate) fn is_evaluating_strict_code(&self) -> bool {
        let Some(strict) = self
            .running_execution_context()
            .ecmascript_code
            .map(|e| e.is_strict_mode)
        else {
            panic_corrupted_agent()
        };
        strict
    }

    pub(crate) fn check_call_depth<'gc>(&mut self, gc: NoGcScope<'gc, '_>) -> JsResult<'gc, ()> {
        // Experimental number that caused stack overflow on local machine. A
        // better limit creation logic would be nice.
        if self.execution_context_stack.len() > 3500 {
            Err(self.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Maximum call stack size exceeded",
                gc,
            ))
        } else {
            Ok(())
        }
    }

    /// Returns the realm of the previous execution context.
    ///
    /// See steps 6-8 of [27.6.3.8 AsyncGeneratorYield ( value )](https://tc39.es/ecma262/#sec-asyncgeneratoryield).
    pub(crate) fn get_previous_context_realm<'a>(&self, gc: NoGcScope<'a, '_>) -> Realm<'a> {
        // 6. Assert: The execution context stack has at least two elements.
        assert!(self.execution_context_stack.len() >= 2);
        // 7. Let previousContext be the second to top element of the execution
        //    context stack.
        let previous_context =
            &self.execution_context_stack[self.execution_context_stack.len() - 2];
        // 8. Let previousRealm be previousContext's Realm.
        previous_context.realm.bind(gc)
    }

    pub(crate) fn push_execution_context(&mut self, context: ExecutionContext) {
        self.execution_context_stack.push(context);
    }

    pub(crate) fn pop_execution_context(&mut self) -> Option<ExecutionContext> {
        self.execution_context_stack.pop()
    }

    pub(crate) fn current_source_code<'a>(&self, gc: NoGcScope<'a, '_>) -> SourceCode<'a> {
        let Some(s) = self
            .execution_context_stack
            .last()
            .and_then(|s| s.ecmascript_code.as_ref())
            .map(|e| e.source_code.bind(gc))
        else {
            panic_corrupted_agent()
        };
        s
    }

    /// Returns the running execution context's LexicalEnvironment.
    pub(crate) fn current_lexical_environment<'a>(&self, gc: NoGcScope<'a, '_>) -> Environment<'a> {
        let Some(e) = self
            .execution_context_stack
            .last()
            .and_then(|s| s.ecmascript_code.as_ref())
            .map(|e| e.lexical_environment.bind(gc))
        else {
            panic_corrupted_agent()
        };
        e
    }

    /// Returns the running execution context's VariableEnvironment.
    pub(crate) fn current_variable_environment<'a>(
        &self,
        gc: NoGcScope<'a, '_>,
    ) -> Environment<'a> {
        let Some(e) = self
            .execution_context_stack
            .last()
            .and_then(|s| s.ecmascript_code.as_ref())
            .map(|e| e.variable_environment.bind(gc))
        else {
            panic_corrupted_agent()
        };
        e
    }

    /// Returns the running execution context's PrivateEnvironment.
    pub(crate) fn current_private_environment<'a>(
        &self,
        gc: NoGcScope<'a, '_>,
    ) -> Option<PrivateEnvironment<'a>> {
        let Some(e) = self
            .execution_context_stack
            .last()
            .and_then(|s| s.ecmascript_code.as_ref())
            .map(|e| e.private_environment.bind(gc))
        else {
            panic_corrupted_agent()
        };
        e
    }

    /// Sets the running execution context's LexicalEnvironment.
    pub(crate) fn set_current_lexical_environment(&mut self, env: Environment) {
        let Some(_) = self
            .execution_context_stack
            .last_mut()
            .and_then(|s| s.ecmascript_code.as_mut())
            .map(|e| {
                e.lexical_environment = env.unbind();
            })
        else {
            panic_corrupted_agent()
        };
    }

    /// Sets the running execution context's VariableEnvironment.
    pub(crate) fn set_current_variable_environment(&mut self, env: Environment) {
        let Some(_) = self
            .execution_context_stack
            .last_mut()
            .and_then(|s| s.ecmascript_code.as_mut())
            .map(|e| {
                e.variable_environment = env.unbind();
            })
        else {
            panic_corrupted_agent()
        };
    }

    /// Sets the running execution context's PrivateEnvironment.
    pub(crate) fn set_current_private_environment(&mut self, env: Option<PrivateEnvironment>) {
        let Some(_) = self
            .execution_context_stack
            .last_mut()
            .and_then(|s| s.ecmascript_code.as_mut())
            .map(|e| {
                e.private_environment = env.unbind();
            })
        else {
            panic_corrupted_agent()
        };
    }

    /// Allocates a range of PrivateName identifiers and returns the first in
    /// the range.
    pub(crate) fn create_private_names(&mut self, count: usize) -> PrivateName {
        let count = u32::try_from(count).expect("Unreasonable amount of PrivateNames");
        let first = self.private_names_counter;
        let next_free_name = first
            .checked_add(count)
            .expect("PrivateName counter overflowed");
        self.private_names_counter = next_free_name;
        PrivateName::from_u32(first)
    }

    /// ### [9.6.3 IncrementModuleAsyncEvaluationCount ( )](https://tc39.es/ecma262/#sec-IncrementModuleAsyncEvaluationCount)
    ///
    /// The abstract operation IncrementModuleAsyncEvaluationCount takes no
    /// arguments and returns an integer.
    ///
    /// > NOTE: This value is only used to keep track of the relative
    /// > evaluation order between pending modules. An implementation may
    /// > unobservably reset \[\[ModuleAsyncEvaluationCount]] to 0 whenever
    /// > there are no pending modules.
    pub(crate) fn increment_module_async_evaluation_count(&mut self) -> u32 {
        // 1. Let AR be the Agent Record of the surrounding agent.
        // 2. Let count be AR.[[ModuleAsyncEvaluationCount]].
        let count = self.module_async_evaluation_count;
        // 3. Set AR.[[ModuleAsyncEvaluationCount]] to count + 1.
        self.module_async_evaluation_count += 1;
        // 4. Return count.
        count
    }

    /// Panics if no active function object exists.
    pub(crate) fn active_function_object<'a>(&self, gc: NoGcScope<'a, '_>) -> Function<'a> {
        let Some(f) = self
            .execution_context_stack
            .last()
            .and_then(|s| s.function.bind(gc))
        else {
            panic_corrupted_agent()
        };
        f
    }

    /// ### [9.4.1 GetActiveScriptOrModule ( )](https://tc39.es/ecma262/#sec-getactivescriptormodule)
    ///
    /// The abstract operation GetActiveScriptOrModule takes no arguments and
    /// returns a Script Record, a Module Record, or null. It is used to
    /// determine the running script or module, based on the running execution
    /// context.
    pub(crate) fn get_active_script_or_module<'a>(
        &self,
        gc: NoGcScope<'a, '_>,
    ) -> Option<ScriptOrModule<'a>> {
        let Some(s) = self
            .execution_context_stack
            .last()
            .map(|s| s.script_or_module.bind(gc))
        else {
            panic_corrupted_agent()
        };
        s
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
    ) -> JsResult<'gc, Value<'gc>> {
        let realm = self.current_realm(gc.nogc());
        let script = match parse_script(self, source_text, realm, false, None, gc.nogc()) {
            Ok(script) => script,
            Err(err) => {
                let gc = gc.into_nogc();
                let message =
                    String::from_string(self, err.first().unwrap().message.to_string(), gc);
                return Err(self.throw_exception_with_message(
                    ExceptionType::SyntaxError,
                    message,
                    gc,
                ));
            }
        };
        script_evaluation(self, script.unbind(), gc)
    }

    /// Run a parsed SourceTextModule in the current Realm.
    ///
    /// This runs the LoadRequestedModules (passing in the host_defined
    /// parameter), Link, and finally Evaluate operations on the module.
    /// This should not be called multiple times on the same module.
    pub fn run_parsed_module<'gc>(
        &mut self,
        module: SourceTextModule,
        host_defined: Option<HostDefined>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let module = module.bind(gc.nogc());
        let Some(result) = module
            .load_requested_modules(self, host_defined, gc.nogc())
            .try_get_result(self, gc.nogc())
        else {
            return Err(self.throw_exception_with_static_message(
                ExceptionType::Error,
                "module was not sync",
                gc.into_nogc(),
            ));
        };
        result.unbind()?;

        module.link(self, gc.nogc()).unbind()?;
        if let Some(result) = module
            .unbind()
            .evaluate(self, gc.reborrow())
            .unbind()
            .try_get_result(self, gc.into_nogc())
        {
            // Note: module resolved synchronously.
            result
        } else {
            Ok(Value::Undefined)
        }
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
    agent
        .execution_context_stack
        .iter()
        .rev()
        .find_map(|context| context.script_or_module)
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
    cache: Option<PropertyLookupCache<'a>>,
    gc: NoGcScope<'a, '_>,
) -> TryResult<'a, Reference<'a>> {
    // 1. If env is not present or env is undefined, then
    // a. Set env to the running execution context's LexicalEnvironment.
    let env = agent.current_lexical_environment(gc);

    // 2. Assert: env is an Environment Record.
    // Implicit from env's type.

    // 3. Let strict be IsStrict(the syntactic production that is being evaluated).
    let strict = agent.is_evaluating_strict_code();

    // 4. Return ? GetIdentifierReference(env, name, strict).
    try_get_identifier_reference(agent, env, name, cache, strict, gc)
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
    cache: Option<PropertyLookupCache<'a>>,
    env: Option<Environment>,
    gc: GcScope<'a, 'b>,
) -> JsResult<'a, Reference<'a>> {
    let name = name.bind(gc.nogc());
    let env = env
        .unwrap_or_else(|| {
            // 1. If env is not present or env is undefined, then
            //    a. Set env to the running execution context's LexicalEnvironment.
            agent.current_lexical_environment(gc.nogc())
        })
        .bind(gc.nogc());
    let cache = cache.bind(gc.nogc());

    // 2. Assert: env is an Environment Record.
    // Implicit from env's type.

    // 3. Let strict be IsStrict(the syntactic production that is being evaluated).
    let strict = agent.is_evaluating_strict_code();

    // 4. Return ? GetIdentifierReference(env, name, strict).
    get_identifier_reference(
        agent,
        Some(env.unbind()),
        name.unbind(),
        cache.unbind(),
        strict,
        gc,
    )
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

impl HeapMarkAndSweep for Agent {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            heap,
            execution_context_stack,
            stack_refs,
            stack_ref_collections,
            vm_stack,
            options: _,
            symbol_id: _,
            global_symbol_registry,
            host_hooks: _,
            #[cfg(feature = "weak-refs")]
                kept_alive: _,
            private_names_counter: _,
            module_async_evaluation_count: _,
        } = self;

        execution_context_stack.iter().for_each(|ctx| {
            ctx.mark_values(queues);
        });
        stack_refs
            .borrow()
            .iter()
            .for_each(|value| value.mark_values(queues));
        stack_ref_collections
            .borrow()
            .iter()
            .for_each(|collection| collection.mark_values(queues));
        vm_stack.iter().for_each(|vm_ptr| {
            unsafe { vm_ptr.as_ref() }.mark_values(queues);
        });
        global_symbol_registry.mark_values(queues);
        let mut last_filled_global_value = None;
        heap.globals
            .borrow()
            .iter()
            .enumerate()
            .for_each(|(i, &value)| {
                if value != HeapRootData::Empty {
                    value.mark_values(queues);
                    last_filled_global_value = Some(i);
                }
            });
        // Remove as many `None` global values without moving any `Some(Value)` values.
        if let Some(last_filled_global_value) = last_filled_global_value {
            heap.globals
                .borrow_mut()
                .drain(last_filled_global_value + 1..);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Agent {
            heap: _,
            execution_context_stack,
            stack_refs,
            stack_ref_collections,
            vm_stack,
            options: _,
            symbol_id: _,
            global_symbol_registry,
            host_hooks: _,
            #[cfg(feature = "weak-refs")]
                kept_alive: _,
            private_names_counter: _,
            module_async_evaluation_count: _,
        } = self;

        execution_context_stack
            .iter_mut()
            .for_each(|entry| entry.sweep_values(compactions));
        stack_refs
            .borrow_mut()
            .iter_mut()
            .for_each(|entry| entry.sweep_values(compactions));
        stack_ref_collections
            .borrow_mut()
            .iter_mut()
            .for_each(|entry| entry.sweep_values(compactions));
        vm_stack
            .iter_mut()
            .for_each(|entry| unsafe { entry.as_mut().sweep_values(compactions) });
        global_symbol_registry.sweep_values(compactions);
    }
}

#[cold]
#[inline(never)]
fn panic_corrupted_agent() -> ! {
    panic!("Agent is corrupted")
}
