// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2.1.6 Cyclic Module Records](https://tc39.es/ecma262/#sec-cyclic-module-records)

use crate::{
    ecmascript::{
        builtins::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        execution::{Agent, JsResult, agent::JsError},
        scripts_and_modules::{
            module::module_semantics::{
                abstract_module_records::AbstractModuleMethods, get_imported_module,
            },
            script::HostDefined,
        },
        types::Value,
    },
    engine::{
        Scoped,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::{
    LoadedModules, ModuleRequest, ModuleRequestRecord, abstract_module_records::AbstractModule,
    source_text_module_records::SourceTextModule,
};

#[derive(Debug, Default, Clone, Copy)]
pub(crate) enum CyclicModuleRecordStatus {
    #[default]
    New,
    Unlinked,
    Linking,
    Linked,
    Evaluating,
    EvaluatingAsync,
    Evaluated,
}

#[derive(Debug, Default)]
pub(crate) struct CyclicModuleRecord<'a> {
    /// ### \[\[Status]]
    ///
    /// Initially new. Transitions to unlinked, linking, linked, evaluating,
    /// possibly evaluating-async, evaluated (in that order) as the module
    /// progresses throughout its lifecycle. evaluating-async indicates this
    /// module is queued to execute on completion of its asynchronous
    /// dependencies or it is a module whose `[[HasTLA]]` field is true that
    /// has been executed and is pending top-level completion.
    status: CyclicModuleRecordStatus,
    /// ### \[\[EvaluationError]]
    ///
    /// a throw completion or empty
    ///
    /// A throw completion representing the exception that occurred during
    /// evaluation. undefined if no exception occurred or if `[[Status]]` is
    /// not evaluated.
    evaluation_error: Option<JsError<'a>>,
    /// ### \[\[DFSIndex]]
    ///
    /// Auxiliary field used during Link and Evaluate only. If `[[Status]]` is
    /// either linking or evaluating, this non-negative number records the
    /// point at which the module was first visited during the depth-first
    /// traversal of the dependency graph.
    dfs_index: Option<u32>,
    /// ### \[\[DFSAncestorIndex]]
    ///
    /// Auxiliary field used during Link and Evaluate only. If `[[Status]]` is
    /// either linking or evaluating, this is either the module's own
    /// `[[DFSIndex]]` or that of an "earlier" module in the same strongly
    /// connected component.
    dfs_ancestor_index: Option<u32>,
    /// ### \[\[RequestedModules]]
    ///
    /// a List of ModuleRequest Records
    ///
    /// A List of the ModuleRequest Records associated with the imports in this
    /// module. The List is in source text occurrence order of the imports.
    ///
    /// Note: The requested module specifiers are borrowed strings pointing to
    /// the source text of the module record.
    requested_modules: Box<[ModuleRequest<'a>]>,
    /// ### \[\[LoadedModules]]
    ///
    /// a List of LoadedModuleRequest Records
    ///
    /// A map from the specifier strings used by the module represented by this
    /// record to request the importation of a module with the relative import
    /// attributes to the resolved Module Record. The list does not contain two
    /// different Records r1 and r2 such that ModuleRequestsEqual(r1, r2) is true.
    loaded_modules: LoadedModules<'a>,
    /// ### \[\[CycleRoot]]
    ///
    /// a Cyclic Module Record or empty
    ///
    /// The first visited module of the cycle, the root DFS ancestor of the
    /// strongly connected component. For a module not in a cycle, this would
    /// be the module itself. Once Evaluate has completed, a module's
    /// `[[DFSAncestorIndex]]` is the `[[DFSIndex]]` of its `[[CycleRoot]]`.
    cycle_root: (),
    /// ### \[\[HasTLA]]
    ///
    /// Whether this module is individually asynchronous (for example, if it's
    /// a Source Text Module Record containing a top-level await). Having an
    /// asynchronous dependency does not mean this field is true. This field
    /// must not change after the module is parsed.
    has_tla: bool,
    /// ### \[\[AsyncEvaluationOrder]]
    ///
    /// unset, an integer, or done
    ///
    /// This field is initially set to unset, and remains unset for fully
    /// synchronous modules. For modules that are either themselves
    /// asynchronous or have an asynchronous dependency, it is set to an
    /// integer that determines the order in which execution of pending modules
    /// is queued by 16.2.1.6.1.3.4. Once the pending module is executed, the
    /// field is set to done.
    async_evaluation_order: Option<()>,
    /// ### \[\[TopLevelCapability]]
    ///
    /// a PromiseCapability Record or empty
    ///
    /// If this module is the `[[CycleRoot]]` of some cycle, and Evaluate() was
    /// called on some module in that cycle, this field contains the
    /// PromiseCapability Record for that entire evaluation. It is used to
    /// settle the Promise object that is returned from the Evaluate() abstract
    /// method. This field will be empty for any dependencies of that module,
    /// unless a top-level Evaluate() has been initiated for some of those
    /// dependencies.
    top_level_capability: Option<PromiseCapability<'a>>,
    /// ### \[\[AsyncParentModules]]
    ///
    /// a List of Cyclic Module Records
    ///
    /// If this module or a dependency has `[[HasTLA]]` true, and execution is
    /// in progress, this tracks the parent importers of this module for the
    /// top-level execution job. These parent modules will not start executing
    /// before this module has successfully completed execution.
    async_parent_modules: (),
    /// ### \[\[PendingAsyncDependencies]]
    ///
    /// If this module has any asynchronous dependencies, this tracks the
    /// number of asynchronous dependency modules remaining to execute for this
    /// module. A module with asynchronous dependencies will be executed when
    /// this field reaches 0 and there are no execution errors.
    pending_async_dependencies: Option<u32>,
}

impl<'m> CyclicModuleRecord<'m> {
    pub(super) fn new(r#async: bool, requested_modules: Box<[ModuleRequest<'m>]>) -> Self {
        Self {
            has_tla: r#async,
            requested_modules,
            ..Default::default()
        }
    }

    /// ### \[\[HasTLA]]
    pub(super) fn has_tla(&self) -> bool {
        self.has_tla
    }

    /// Get a loaded module by module request reference.
    pub(super) fn get_loaded_module(
        &self,
        agent: &Agent,
        request: ModuleRequest<'m>,
    ) -> Option<AbstractModule<'m>> {
        self.loaded_modules
            .get_loaded_module(agent.as_ref(), request)
    }

    /// Insert a loaded module into the module's requested modules.
    pub(super) fn insert_loaded_module(
        &mut self,
        requests: &Vec<ModuleRequestRecord<'static>>,
        request: ModuleRequest<'m>,
        module: AbstractModule<'m>,
    ) {
        self.loaded_modules
            .insert_loaded_module(requests, request, module);
    }

    /// Get the requested modules as a slice.
    pub(super) fn get_requested_modules(&self) -> &[ModuleRequest<'m>] {
        &self.requested_modules
    }

    /// ### \[\[EvaluationError]]
    pub(super) fn evaluation_error<'gc>(&self, gc: NoGcScope<'gc, '_>) -> JsResult<'gc, ()> {
        if let Some(error) = self.evaluation_error {
            Err(error.bind(gc))
        } else {
            Ok(())
        }
    }

    /// Set \[\[EvaluationError]] to error and \[\[Status]] to evaluated.
    pub(super) fn set_evaluation_error(&mut self, error: JsError) {
        debug_assert!(
            self.evaluation_error.is_none(),
            "Attempted to set module [[EvaluationError]] twice"
        );
        debug_assert!(matches!(self.status, CyclicModuleRecordStatus::Evaluating));
        self.evaluation_error = Some(error.unbind());
        self.status = CyclicModuleRecordStatus::Evaluated;
    }

    /// ### \[\[Status]]
    pub(super) fn status(&self) -> CyclicModuleRecordStatus {
        self.status
    }

    /// ### \[\[TopLevelCapability]]
    pub(super) fn top_level_capability(&self) -> Option<&PromiseCapability<'m>> {
        self.top_level_capability.as_ref()
    }

    /// ### \[\[DFSAncestorIndex]]
    pub(super) fn dfs_ancestor_index(&self) -> u32 {
        self.dfs_ancestor_index
            .expect("Attempted to get [[DFSAncestorIndex]] of new module")
    }

    /// Set \[\[DFSAncestorIndex]] to value if it is larger than before.
    pub(super) fn set_dfs_ancestor_index(&mut self, value: u32) {
        let dfs_ancestor_index = self
            .dfs_ancestor_index
            .as_mut()
            .expect("Attempted to set [[DFSAncestorIndex]] of new module");
        *dfs_ancestor_index = (*dfs_ancestor_index).max(value);
    }

    /// ### \[\[DFSIndex]]
    pub(super) fn dfs_index(&self) -> u32 {
        self.dfs_index
            .expect("Attempted to get [[DFSIndex]] of new module")
    }

    /// Set \[\[DFSIndex]] and \[\[DFSAncestorIndex]] to index.
    pub(super) fn set_dfs_index(&mut self, index: u32) {
        self.dfs_index = Some(index);
        self.dfs_ancestor_index = Some(index);
    }

    /// Set module.\[\[Status]] to unlinked.
    pub(super) fn set_unlinked(&mut self) {
        debug_assert!(matches!(
            self.status,
            CyclicModuleRecordStatus::New | CyclicModuleRecordStatus::Linking
        ));
        self.status = CyclicModuleRecordStatus::Unlinked;
    }

    /// Set module.\[\[Status]] to linking.
    pub(super) fn set_linking(&mut self) {
        debug_assert!(matches!(self.status, CyclicModuleRecordStatus::Unlinked));
        self.status = CyclicModuleRecordStatus::Linking;
    }

    /// Set module.\[\[Status]] to linked.
    pub(super) fn set_linked(&mut self) {
        debug_assert!(matches!(self.status, CyclicModuleRecordStatus::Linking));
        self.status = CyclicModuleRecordStatus::Linked;
    }

    /// Set module.\[\[Status]] to evaluating.
    pub(super) fn set_evaluating(&mut self) {
        debug_assert!(matches!(self.status, CyclicModuleRecordStatus::Linked));
        self.status = CyclicModuleRecordStatus::Evaluating;
    }

    /// Set module.\[\[Status]] to evaluated.
    pub(super) fn set_evaluated(&mut self) {
        debug_assert!(matches!(
            self.status,
            CyclicModuleRecordStatus::Linked
                | CyclicModuleRecordStatus::Evaluating
                | CyclicModuleRecordStatus::EvaluatingAsync
        ));
        self.status = CyclicModuleRecordStatus::Evaluated;
    }
}

/// ### [16.2.1.6 Cyclic Module Records](https://tc39.es/ecma262/#sec-cyclic-module-records)
#[repr(transparent)]
pub struct CyclicModule<'a>(InnerCyclicModule<'a>);

// SAFETY: Pass-through
unsafe impl Bindable for CyclicModule<'_> {
    type Of<'a> = CyclicModule<'a>;

    fn unbind(self) -> Self::Of<'static> {
        CyclicModule(self.0.unbind())
    }

    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        CyclicModule(self.0.bind(gc))
    }
}

pub(crate) enum InnerCyclicModule<'a> {
    SourceTextModule(SourceTextModule<'a>),
}

// SAFETY: Pass-through.
unsafe impl Bindable for InnerCyclicModule<'_> {
    type Of<'a> = InnerCyclicModule<'a>;

    fn unbind(self) -> Self::Of<'static> {
        match self {
            Self::SourceTextModule(m) => InnerCyclicModule::SourceTextModule(m.unbind()),
        }
    }

    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        match self {
            Self::SourceTextModule(m) => InnerCyclicModule::SourceTextModule(m.bind(gc)),
        }
    }
}

pub(crate) trait CyclicModuleSlots: Copy {
    /// ### \[\[Status]]
    fn status(self, agent: &Agent) -> CyclicModuleRecordStatus;
}

/// ### [Additional Abstract Methods of Cyclic Module Records](https://tc39.es/ecma262/#table-cyclic-module-methods)
pub(crate) trait CyclicModuleMethods: CyclicModuleSlots {
    /// ### InitializeEnvironment()
    ///
    /// Initialize the Environment Record of the module, including resolving
    /// all imported bindings, and create the module's execution context.
    fn initialize_environment<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()>;

    /// ### InitializeEnvironment()
    ///
    /// Note: This implements a custom step to bind constant value imports into
    /// the module environment after imported modules have been executed. This
    /// allows us to skip one indirection for imported values.
    ///
    /// Note: let bindings will need the indirection separately.
    fn bind_environment(self, agent: &mut Agent, gc: NoGcScope);

    /// ### ExecuteModule(\[promiseCapability])
    ///
    /// Evaluate the module's code within its execution context. If this module
    /// has true in \[\[HasTLA]], then a PromiseCapability Record is passed as
    /// an argument, and the method is expected to resolve or reject the given
    /// capability. In this case, the method must not throw an exception, but
    /// instead reject the PromiseCapability Record if necessary.
    fn execute_module<'a>(
        self,
        agent: &mut Agent,
        promise_capability: Option<PromiseCapability>,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()>;
}

#[derive(Debug)]
pub struct GraphLoadingStateRecord<'a> {
    // ### \[\[PromiseCapability]]
    //
    // a PromiseCapability Record
    //
    // The promise to resolve when the loading process finishes.
    pub(super) promise_capability: PromiseCapability<'a>,
    // ### \[\[IsLoading]]
    //
    // a Boolean
    //
    // It is true if the loading process has not finished yet, neither successfully nor with an error.
    pub(super) is_loading: bool,
    // ### \[\[PendingModulesCount]]
    //
    // a non-negative integer
    //
    // It tracks the number of pending HostLoadImportedModule calls.
    pub(super) pending_modules_count: u32,
    // ### \[\[Visited]]
    //
    // a List of Cyclic Module Records
    //
    // It is a list of the Cyclic Module Records that have been already loaded by the current loading process, to avoid infinite loops with circular dependencies.
    pub(super) visited: Vec<SourceTextModule<'a>>,
    // ### \[\[HostDefined]]
    //
    // anything (default value is empty)
    //
    // It contains host-defined data to pass from the LoadRequestedModules caller to HostLoadImportedModule.
    pub(super) host_defined: Option<HostDefined>,
}

/// ### [16.2.1.6.1.1.1 InnerModuleLoading ( state, module )](https://tc39.es/ecma262/#sec-InnerModuleLoading)
///
/// The abstract operation InnerModuleLoading takes arguments state (a
/// GraphLoadingState Record) and module (a Module Record) and returns unused.
/// It is used by LoadRequestedModules to recursively perform the actual
/// loading process for module's dependency graph.
pub(super) fn inner_module_loading<'a>(
    agent: &mut Agent,
    state: &mut GraphLoadingStateRecord<'a>,
    module: AbstractModule<'a>,
    gc: NoGcScope<'a, '_>,
) {
    // 1. Assert: state.[[IsLoading]] is true.
    debug_assert!(state.is_loading);
    // 2. If module is a Cyclic Module Record, module.[[Status]] is new, and state.[[Visited]] does not contain module, then
    if let Some(module) = module.as_source_text_module() {
        if matches!(module.status(agent), CyclicModuleRecordStatus::New)
            && !state.visited.contains(&module)
        {
            // a. Append module to state.[[Visited]].
            state.visited.push(module);
            // b. Let requestedModulesCount be the number of elements in module.[[RequestedModules]].
            // SAFETY: No GC in this scope.
            let requested_modules = unsafe { module.get_requested_modules(agent) };
            let requested_module_count = requested_modules.len() as u32;
            // c. Set state.[[PendingModulesCount]] to state.[[PendingModulesCount]] + requestedModulesCount.
            state.pending_modules_count += requested_module_count;
            // d. For each ModuleRequest Record request of module.[[RequestedModules]], do
            for request in requested_modules {
                // i. If AllImportAttributesSupported(request.[[Attributes]]) is false, then
                //         1. Let error be ThrowCompletion(a newly created SyntaxError object).
                //         2. Perform ContinueModuleLoading(state, error).
                // ii. Else if module.[[LoadedModules]] contains a LoadedModuleRequest Record
                //     record such that ModuleRequestsEqual(record, request) is true, then
                //         1. Perform InnerModuleLoading(state, record.[[Module]]).
                // iii. Else,
                // 1. Perform HostLoadImportedModule(module, request, state.[[HostDefined]], state).
                agent.host_hooks.load_imported_module(
                    agent,
                    module.into(),
                    *request,
                    state.host_defined.clone(),
                    state,
                    gc,
                );
                // 2. NOTE: HostLoadImportedModule will call FinishLoadingImportedModule,
                //    which re-enters the graph loading process through ContinueModuleLoading.
                // iv. If state.[[IsLoading]] is false,
                if !state.is_loading {
                    // return unused.
                    return;
                }
            }
        }
    }
    // 3. Assert: state.[[PendingModulesCount]] ≥ 1.
    debug_assert!(state.pending_modules_count >= 1);
    // 4. Set state.[[PendingModulesCount]] to state.[[PendingModulesCount]] - 1.
    state.pending_modules_count -= 1;
    // 5. If state.[[PendingModulesCount]] = 0, then
    if state.pending_modules_count == 0 {
        // a. Set state.[[IsLoading]] to false.
        state.is_loading = false;
        // b. For each Cyclic Module Record loaded of state.[[Visited]], do
        for loaded in state.visited.drain(..) {
            // i. If loaded.[[Status]] is new, set loaded.[[Status]] to unlinked.
            if matches!(loaded.status(agent), CyclicModuleRecordStatus::New) {
                loaded.set_unlinked(agent);
            }
        }
        // c. Perform ! Call(state.[[PromiseCapability]].[[Resolve]], undefined, « undefined »).
        state
            .promise_capability
            .internal_fulfill(agent, Value::Undefined, gc);
    }
    // 6. Return unused.
}

/// ### [16.2.1.6.1.2.1 InnerModuleLinking ( module, stack, index )](https://tc39.es/ecma262/#sec-InnerModuleLinking)
///
/// The abstract operation InnerModuleLinking takes arguments module (a Module
/// Record), stack (a List of Cyclic Module Records), and index (a non-negative
/// integer) and returns either a normal completion containing a non-negative
/// integer or a throw completion. It is used by Link to perform the actual
/// linking process for module, as well as recursively on all other modules in
/// the dependency graph. The stack and index parameters, as well as a module's
/// \[\[DFSIndex]] and \[\[DFSAncestorIndex]] fields, keep track of the
/// depth-first search (DFS) traversal. In particular, \[\[DFSAncestorIndex]]
/// is used to discover strongly connected components (SCCs), such that all
/// modules in an SCC transition to linked together.
pub(super) fn inner_module_linking<'a>(
    agent: &mut Agent,
    module: AbstractModule<'a>,
    stack: &mut Vec<SourceTextModule<'a>>,
    index: u32,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, u32> {
    let module = module.bind(gc);
    // 1. If module is not a Cyclic Module Record, then
    let Some(module) = module.as_source_text_module() else {
        // a. Perform ? module.Link().
        module.link(agent, gc)?;
        // b. Return index.
        return Ok(index);
    };
    // 2. If module.[[Status]] is one of linking, linked, evaluating-async, or evaluated, then
    if matches!(
        module.status(agent),
        CyclicModuleRecordStatus::Linking
            | CyclicModuleRecordStatus::Linked
            | CyclicModuleRecordStatus::EvaluatingAsync
            | CyclicModuleRecordStatus::Evaluated
    ) {
        // a. Return index.
        return Ok(index);
    }
    // 3. Assert: module.[[Status]] is unlinked.
    debug_assert!(matches!(
        module.status(agent),
        CyclicModuleRecordStatus::Unlinked
    ));
    // 4. Set module.[[Status]] to linking.
    module.set_linking(agent);
    // 5. Set module.[[DFSIndex]] to index.
    // 6. Set module.[[DFSAncestorIndex]] to index.
    module.set_dfs_index(agent, index);
    // 7. Set index to index + 1.
    // Note: if this overflows, we have worse issues.
    let (mut index, _) = index.overflowing_add(1);
    // 8. Append module to stack.
    stack.push(module);
    // 9. For each ModuleRequest Record request of module.[[RequestedModules]], do
    // SAFETY: module is currently rooted.
    for request in unsafe { module.get_requested_modules(agent) } {
        // a. Let requiredModule be GetImportedModule(module, request).
        let required_module = get_imported_module(agent, module, *request, gc);
        // b. Set index to ? InnerModuleLinking(requiredModule, stack, index).
        index = inner_module_linking(agent, required_module, stack, index, gc)?;
        // c. If requiredModule is a Cyclic Module Record, then
        if let Some(required_module) = required_module.as_source_text_module() {
            // i. Assert: requiredModule.[[Status]] is one of linking, linked,
            //    evaluating-async, or evaluated.
            debug_assert!(matches!(
                required_module.status(agent),
                CyclicModuleRecordStatus::Linking
                    | CyclicModuleRecordStatus::Linked
                    | CyclicModuleRecordStatus::EvaluatingAsync
                    | CyclicModuleRecordStatus::Evaluated
            ));
            // ii. Assert: requiredModule.[[Status]] is linking if and only if
            //     stack contains requiredModule.
            // iii. If requiredModule.[[Status]] is linking, then
            if matches!(
                required_module.status(agent),
                CyclicModuleRecordStatus::Linking
            ) {
                debug_assert!(stack.contains(&required_module));
                // 1. Set module.[[DFSAncestorIndex]] to
                //    min(module.[[DFSAncestorIndex]],
                //    requiredModule.[[DFSAncestorIndex]]).
                module.set_dfs_ancestor_index(agent, required_module.dfs_ancestor_index(agent));
            }
        }
    }
    // 10. Perform ? module.InitializeEnvironment().
    module.initialize_environment(agent, gc)?;
    // 11. Assert: module occurs exactly once in stack.
    debug_assert!(stack.iter().filter(|m| **m == module).count() == 1);
    // 12. Assert: module.[[DFSAncestorIndex]] ≤ module.[[DFSIndex]].
    debug_assert!(module.dfs_ancestor_index(agent) <= module.dfs_index(agent));
    // 13. If module.[[DFSAncestorIndex]] = module.[[DFSIndex]], then
    if module.dfs_ancestor_index(agent) == module.dfs_index(agent) {
        // a. Let done be false.
        // b. Repeat, while done is false,
        while let Some(required_module) = stack.pop() {
            // i. Let requiredModule be the last element of stack.
            // ii. Remove the last element of stack.
            // iii. Assert: requiredModule is a Cyclic Module Record.
            // iv. Set requiredModule.[[Status]] to linked.
            required_module.set_linked(agent);
            // v. If requiredModule and module are the same Module Record, set done to true.
            if required_module == module {
                break;
            }
        }
    }
    // 14. Return index.
    Ok(index)
}

/// ### [16.2.1.6.1.3.1 InnerModuleEvaluation ( module, stack, index )](https://tc39.es/ecma262/#sec-innermoduleevaluation)
///
/// The abstract operation InnerModuleEvaluation takes arguments module (a
/// Module Record), stack (a List of Cyclic Module Records), and index (a
/// non-negative integer) and returns either a normal completion containing a
/// non-negative integer or a throw completion. It is used by Evaluate to
/// perform the actual evaluation process for module, as well as recursively on
/// all other modules in the dependency graph. The stack and index parameters,
/// as well as module's \[\[DFSIndex]] and \[\[DFSAncestorIndex]] fields, are
/// used the same way as in InnerModuleLinking.
///
/// > NOTE 1: A module is evaluating while it is being traversed by
/// > InnerModuleEvaluation. A module is evaluated on execution completion or
/// > evaluating-async during execution if its \[\[HasTLA]] field is true or if
/// > it has asynchronous dependencies.
///
/// > NOTE 2: Any modules depending on a module of an asynchronous cycle when
/// > that cycle is not evaluating will instead depend on the execution of the
/// > root of the cycle via \[\[CycleRoot]]. This ensures that the cycle state
/// > can be treated as a single strongly connected component through its root
/// > module state.
pub(super) fn inner_module_evaluation<'a, 'b>(
    agent: &mut Agent,
    scoped_module: Scoped<'b, AbstractModule<'static>>,
    stack: &mut Vec<Scoped<'b, SourceTextModule<'static>>>,
    mut index: u32,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<'a, u32> {
    let module = scoped_module.get(agent).bind(gc.nogc());
    // 1. If module is not a Cyclic Module Record, then
    let Some(mut module) = module.as_source_text_module() else {
        // a. Perform ? EvaluateModuleSync(module).
        // evaluate_module_sync(agent, module, gc)?;
        module.unbind().evaluate(agent, gc.reborrow());
        // b. Return index.
        return Ok(index);
    };
    // SAFETY: We're not actually replacing anything but just
    // reinterpreting the Scoped inner type.
    let scoped_module = unsafe { scoped_module.replace_self(agent, module.unbind()) };
    // 2. If module.[[Status]] is either evaluating-async or evaluated, then
    if matches!(
        module.status(agent),
        CyclicModuleRecordStatus::EvaluatingAsync | CyclicModuleRecordStatus::Evaluated
    ) {
        // a. If module.[[EvaluationError]] is empty, return index.
        module.unbind().evaluation_error(agent, gc.into_nogc())?;
        // b. Otherwise, return ? module.[[EvaluationError]].
        return Ok(index);
    }
    // 3. If module.[[Status]] is evaluating,
    if matches!(module.status(agent), CyclicModuleRecordStatus::Evaluating) {
        // return index.
        return Ok(index);
    }
    // 4. Assert: module.[[Status]] is linked.
    assert!(matches!(
        module.status(agent),
        CyclicModuleRecordStatus::Linked
    ));
    // 5. Set module.[[Status]] to evaluating.
    module.set_evaluating(agent);
    // 6. Set module.[[DFSIndex]] to index.
    // 7. Set module.[[DFSAncestorIndex]] to index.
    module.set_dfs_index(agent, index);
    // 8. Set module.[[PendingAsyncDependencies]] to 0.
    // 9. Set index to index + 1.
    index += 1;
    // 10. Append module to stack.
    stack.push(scoped_module.clone());
    // 11. For each ModuleRequest Record request of module.[[RequestedModules]], do
    // SAFETY: module is currently rooted.
    for request in scoped_module.get_requested_modules(agent) {
        // a. Let requiredModule be GetImportedModule(module, request).
        let required_module: AbstractModule =
            get_imported_module(agent, module, request, gc.nogc()).into();

        let scoped_required_module = required_module.scope(agent, gc.nogc());
        // b. Set index to ? InnerModuleEvaluation(requiredModule, stack, index).
        index = inner_module_evaluation(
            agent,
            scoped_required_module.clone(),
            stack,
            index,
            gc.reborrow(),
        )
        .unbind()?;
        module = scoped_module.get(agent).bind(gc.nogc());
        let required_module = scoped_required_module.get(agent).bind(gc.nogc());
        // c. If requiredModule is a Cyclic Module Record, then
        if let Some(required_module) = required_module.as_source_text_module() {
            // i. Assert: requiredModule.[[Status]] is one of evaluating,
            //    evaluating-async, or evaluated.
            debug_assert!(matches!(
                required_module.status(agent),
                CyclicModuleRecordStatus::Evaluating
                    | CyclicModuleRecordStatus::EvaluatingAsync
                    | CyclicModuleRecordStatus::Evaluated
            ));
            // ii. Assert: requiredModule.[[Status]] is evaluating if and only if stack contains requiredModule.
            // iii. If requiredModule.[[Status]] is evaluating, then
            if matches!(
                required_module.status(agent),
                CyclicModuleRecordStatus::Evaluating
            ) {
                debug_assert!(stack.iter().any(|m| m.get(agent) == required_module));
                // 1. Set module.[[DFSAncestorIndex]] to min(module.[[DFSAncestorIndex]], requiredModule.[[DFSAncestorIndex]]).
                module.set_dfs_ancestor_index(agent, required_module.dfs_ancestor_index(agent));
            } else {
                // iv. Else,
                // 1. Set requiredModule to requiredModule.[[CycleRoot]].
                // 2. Assert: requiredModule.[[Status]] is either evaluating-async or evaluated.
                debug_assert!(matches!(
                    required_module.status(agent),
                    CyclicModuleRecordStatus::EvaluatingAsync | CyclicModuleRecordStatus::Evaluated
                ));
                // 3. If requiredModule.[[EvaluationError]] is not empty, return ? requiredModule.[[EvaluationError]].
                required_module
                    .evaluation_error(agent, gc.nogc())
                    .unbind()?;
            }
            // v. If requiredModule.[[AsyncEvaluationOrder]] is an integer, then
            //         1. Set module.[[PendingAsyncDependencies]] to module.[[PendingAsyncDependencies]] + 1.
            //         2. Append module to requiredModule.[[AsyncParentModules]].
        }
    }
    // 12. If module.[[PendingAsyncDependencies]] > 0 or module.[[HasTLA]] is true, then
    //         a. Assert: module.[[AsyncEvaluationOrder]] is unset.
    //         b. Set module.[[AsyncEvaluationOrder]] to IncrementModuleAsyncEvaluationCount().
    //         c. If module.[[PendingAsyncDependencies]] = 0, perform ExecuteAsyncModule(module).
    // 13. Else,
    //         a. Perform ? module.ExecuteModule().
    module.bind_environment(agent, gc.nogc());
    module
        .unbind()
        .execute_module(agent, None, gc.reborrow())
        .unbind()?;
    module = scoped_module.get(agent).bind(gc.nogc());
    // 14. Assert: module occurs exactly once in stack.
    debug_assert_eq!(stack.iter().filter(|m| m.get(agent) == module).count(), 1);
    // 15. Assert: module.[[DFSAncestorIndex]] ≤ module.[[DFSIndex]].
    debug_assert!(module.dfs_ancestor_index(agent) <= module.dfs_index(agent));
    // 16. If module.[[DFSAncestorIndex]] = module.[[DFSIndex]], then
    if module.dfs_ancestor_index(agent) == module.dfs_index(agent) {
        // a. Let done be false.
        // b. Repeat, while done is false,
        while let Some(required_module) = stack.pop() {
            let required_module = required_module.get(agent).bind(gc.nogc());
            // i. Let requiredModule be the last element of stack.
            // ii. Remove the last element of stack.
            // iii. Assert: requiredModule is a Cyclic Module Record.
            // iv. Assert: requiredModule.[[AsyncEvaluationOrder]] is either an
            //     integer or unset.
            // v. If requiredModule.[[AsyncEvaluationOrder]] is unset, set
            //    requiredModule.[[Status]] to evaluated.
            required_module.set_evaluated(agent);
            // vi. Otherwise, set requiredModule.[[Status]] to evaluating-async.
            // vii. If requiredModule and module are the same Module Record,
            //      set done to true.
            // viii. Set requiredModule.[[CycleRoot]] to module.
            if required_module == module {
                break;
            }
        }
    }
    // 17. Return index.
    Ok(index)
}

/// ### [16.2.1.6.1.1.2 ContinueModuleLoading ( state, moduleCompletion )](https://tc39.es/ecma262/#sec-ContinueModuleLoading)
///
/// The abstract operation ContinueModuleLoading takes arguments state (a
/// GraphLoadingState Record) and moduleCompletion (either a normal completion
/// containing a Module Record or a throw completion) and returns unused. It is
/// used to re-enter the loading process after a call to
/// HostLoadImportedModule.
pub(super) fn continue_module_loading<'a>(
    agent: &mut Agent,
    state: &mut GraphLoadingStateRecord<'a>,
    module_completion: JsResult<AbstractModule<'a>>,
    gc: NoGcScope<'a, '_>,
) {
    // 1. If state.[[IsLoading]] is false,
    if !state.is_loading {
        // return unused.
        return;
    }
    // 2. If moduleCompletion is a normal completion, then
    match module_completion {
        Ok(value) => {
            // a. Perform InnerModuleLoading(state, moduleCompletion.[[Value]]).
            inner_module_loading(agent, state, value, gc);
        }
        Err(value) => {
            // 3. Else,
            // a. Set state.[[IsLoading]] to false.
            state.is_loading = false;
            // b. Perform ! Call(state.[[PromiseCapability]].[[Reject]], undefined, « moduleCompletion.[[Value]] »).
            state.promise_capability.reject(agent, value.value(), gc);
        }
    }
    // 4. Return unused.
}

impl HeapMarkAndSweep for CyclicModuleRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            status: _,
            evaluation_error,
            dfs_index: _,
            dfs_ancestor_index: _,
            requested_modules,
            loaded_modules,
            cycle_root: _,
            has_tla: _,
            async_evaluation_order: _,
            top_level_capability,
            async_parent_modules: _,
            pending_async_dependencies: _,
        } = self;
        evaluation_error.mark_values(queues);
        requested_modules.mark_values(queues);
        loaded_modules.mark_values(queues);
        top_level_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            status: _,
            evaluation_error,
            dfs_index: _,
            dfs_ancestor_index: _,
            requested_modules,
            loaded_modules,
            cycle_root: _,
            has_tla: _,
            async_evaluation_order: _,
            top_level_capability,
            async_parent_modules: _,
            pending_async_dependencies: _,
        } = self;
        evaluation_error.sweep_values(compactions);
        requested_modules.sweep_values(compactions);
        loaded_modules.sweep_values(compactions);
        top_level_capability.sweep_values(compactions);
    }
}
