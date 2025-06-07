// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2.1.6 Cyclic Module Records](https://tc39.es/ecma262/#sec-cyclic-module-records)

use crate::{
    ecmascript::{
        builtins::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        execution::{Agent, JsResult, agent::JsError},
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::source_text_module_records::SourceTextModule;

#[derive(Debug, Default)]
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
    requested_modules: (),
    /// ### \[\[LoadedModules]]
    ///
    /// a List of LoadedModuleRequest Records
    ///
    /// A map from the specifier strings used by the module represented by this
    /// record to request the importation of a module with the relative import
    /// attributes to the resolved Module Record. The list does not contain two
    /// different Records r1 and r2 such that ModuleRequestsEqual(r1, r2) is true.
    loaded_modules: (),
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
    pub(super) fn new(r#async: bool, requested_modules: ()) -> Self {
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
    pub(super) fn status(&self) -> &CyclicModuleRecordStatus {
        &self.status
    }

    /// ### \[\[TopLevelCapability]]
    pub(super) fn top_level_capability(&self) -> Option<&PromiseCapability<'m>> {
        self.top_level_capability.as_ref()
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
}

/// ### [Additional Abstract Methods of Cyclic Module Records](https://tc39.es/ecma262/#table-cyclic-module-methods)
pub trait CyclicModuleAbstractMethods {
    /// ### InitializeEnvironment()
    ///
    /// Initialize the Environment Record of the module, including resolving
    /// all imported bindings, and create the module's execution context.
    fn initialize_environment<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()>;

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
    module: SourceTextModule,
    _stack: (),
    index: u32,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, u32> {
    let module = module.bind(gc);
    // 1. If module is not a Cyclic Module Record, then
    //         a. Perform ? module.Link().
    //         b. Return index.
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
    let (index, _) = index.overflowing_add(1);
    // 8. Append module to stack.
    // 9. For each ModuleRequest Record request of module.[[RequestedModules]], do
    //         a. Let requiredModule be GetImportedModule(module, request).
    //         b. Set index to ? InnerModuleLinking(requiredModule, stack, index).
    //         c. If requiredModule is a Cyclic Module Record, then
    //                 i. Assert: requiredModule.[[Status]] is one of linking, linked, evaluating-async, or evaluated.
    //                 ii. Assert: requiredModule.[[Status]] is linking if and only if stack contains requiredModule.
    //                 iii. If requiredModule.[[Status]] is linking, then
    //                         1. Set module.[[DFSAncestorIndex]] to min(module.[[DFSAncestorIndex]], requiredModule.[[DFSAncestorIndex]]).
    // 10. Perform ? module.InitializeEnvironment().
    module.initialize_environment(agent, gc)?;
    // 11. Assert: module occurs exactly once in stack.
    // 12. Assert: module.[[DFSAncestorIndex]] ≤ module.[[DFSIndex]].
    // 13. If module.[[DFSAncestorIndex]] = module.[[DFSIndex]], then
    //         a. Let done be false.
    //         b. Repeat, while done is false,
    //                 i. Let requiredModule be the last element of stack.
    //                 ii. Remove the last element of stack.
    //                 iii. Assert: requiredModule is a Cyclic Module Record.
    //                 iv. Set requiredModule.[[Status]] to linked.
    module.set_linked(agent);
    //                 v. If requiredModule and module are the same Module Record, set done to true.
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
pub(super) fn inner_module_evaluation<'a>(
    agent: &mut Agent,
    module: SourceTextModule,
    _stack: (),
    index: u32,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, u32> {
    // 1. If module is not a Cyclic Module Record, then
    //         a. Perform ? EvaluateModuleSync(module).
    //         b. Return index.
    // 2. If module.[[Status]] is either evaluating-async or evaluated, then
    if matches!(
        module.status(agent),
        CyclicModuleRecordStatus::EvaluatingAsync | CyclicModuleRecordStatus::Evaluated
    ) {
        // a. If module.[[EvaluationError]] is empty, return index.
        module.evaluation_error(agent, gc.into_nogc())?;
        // b. Otherwise, return ? module.[[EvaluationError]].
        return Ok(index);
    }
    // 3. If module.[[Status]] is evaluating, return index.
    if matches!(module.status(agent), CyclicModuleRecordStatus::Evaluating) {
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
    // 8. Set module.[[PendingAsyncDependencies]] to 0.
    // 9. Set index to index + 1.
    // 10. Append module to stack.
    // 11. For each ModuleRequest Record request of module.[[RequestedModules]], do
    //         a. Let requiredModule be GetImportedModule(module, request).
    //         b. Set index to ? InnerModuleEvaluation(requiredModule, stack, index).
    //         c. If requiredModule is a Cyclic Module Record, then
    //                 i. Assert: requiredModule.[[Status]] is one of evaluating, evaluating-async, or evaluated.
    //                 ii. Assert: requiredModule.[[Status]] is evaluating if and only if stack contains requiredModule.
    //                 iii. If requiredModule.[[Status]] is evaluating, then
    //                         1. Set module.[[DFSAncestorIndex]] to min(module.[[DFSAncestorIndex]], requiredModule.[[DFSAncestorIndex]]).
    //                 iv. Else,
    //                         1. Set requiredModule to requiredModule.[[CycleRoot]].
    //                         2. Assert: requiredModule.[[Status]] is either evaluating-async or evaluated.
    //                         3. If requiredModule.[[EvaluationError]] is not empty, return ? requiredModule.[[EvaluationError]].
    //                 v. If requiredModule.[[AsyncEvaluationOrder]] is an integer, then
    //                         1. Set module.[[PendingAsyncDependencies]] to module.[[PendingAsyncDependencies]] + 1.
    //                         2. Append module to requiredModule.[[AsyncParentModules]].
    // 12. If module.[[PendingAsyncDependencies]] > 0 or module.[[HasTLA]] is true, then
    //         a. Assert: module.[[AsyncEvaluationOrder]] is unset.
    //         b. Set module.[[AsyncEvaluationOrder]] to IncrementModuleAsyncEvaluationCount().
    //         c. If module.[[PendingAsyncDependencies]] = 0, perform ExecuteAsyncModule(module).
    // 13. Else,
    //         a. Perform ? module.ExecuteModule().
    module.execute_module(agent, None, gc)?;
    // 14. Assert: module occurs exactly once in stack.
    // 15. Assert: module.[[DFSAncestorIndex]] ≤ module.[[DFSIndex]].
    // 16. If module.[[DFSAncestorIndex]] = module.[[DFSIndex]], then
    //         a. Let done be false.
    //         b. Repeat, while done is false,
    //                 i. Let requiredModule be the last element of stack.
    //                 ii. Remove the last element of stack.
    //                 iii. Assert: requiredModule is a Cyclic Module Record.
    //                 iv. Assert: requiredModule.[[AsyncEvaluationOrder]] is either an integer or unset.
    //                 v. If requiredModule.[[AsyncEvaluationOrder]] is unset, set requiredModule.[[Status]] to evaluated.
    //                 vi. Otherwise, set requiredModule.[[Status]] to evaluating-async.
    //                 vii. If requiredModule and module are the same Module Record, set done to true.
    //                 viii. Set requiredModule.[[CycleRoot]] to module.
    // 17. Return index.
    Ok(index)
}

impl HeapMarkAndSweep for CyclicModuleRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            status: _,
            evaluation_error,
            dfs_index: _,
            dfs_ancestor_index: _,
            requested_modules: _,
            loaded_modules: _,
            cycle_root: _,
            has_tla: _,
            async_evaluation_order: _,
            top_level_capability,
            async_parent_modules: _,
            pending_async_dependencies: _,
        } = self;
        evaluation_error.mark_values(queues);
        top_level_capability.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            status: _,
            evaluation_error,
            dfs_index: _,
            dfs_ancestor_index: _,
            requested_modules: _,
            loaded_modules: _,
            cycle_root: _,
            has_tla: _,
            async_evaluation_order: _,
            top_level_capability,
            async_parent_modules: _,
            pending_async_dependencies: _,
        } = self;
        evaluation_error.sweep_values(compactions);
        top_level_capability.sweep_values(compactions);
    }
}
