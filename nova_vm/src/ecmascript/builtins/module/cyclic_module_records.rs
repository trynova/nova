use std::any::Any;

use oxc_span::Atom;

use crate::ecmascript::{
    abstract_operations::operations_on_objects::call_function,
    builtins::{control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability, create_builtin_function, error::Error, module::source_text_module_records::get_imported_module, promise::Promise, ArgumentsList},
    execution::{agent::JsError, Agent, JsResult},
    types::{String, Value},
};

use super::{
    abstract_module_records::{ModuleRecord, NotLoadedErr, ResolvedBinding},
    Module,
};

/// ### [CyclicModuleRecord] \[\[EvaluationError\]\]
///
/// A throw completion representing the exception that occurred during
/// evaluation. undefined if no exception occurred or if \[\[Status\]\] is not
/// evaluated.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EvaluationError(JsError);

/// ### [CyclicModuleRecord] \[\[DFSIndex\]\]
///
/// Auxiliary field used during Link and Evaluate only. If \[\[Status\]\] is
/// either linking or evaluating, this non-negative number records the point at
/// which the module was first visited during the depth-first traversal of the
/// dependency graph.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DFSIndex(u16);

impl DFSIndex {
    const fn new(index: u16) -> Self {
        Self(index)
    }

    const fn value(&self) -> u16 {
        self.0
    }
}

/// ### [CyclicModuleRecord] \[\[DFSAncestorIndex\]\]
///
/// Auxiliary field used during Link and Evaluate only. If \[\[Status\]\] is
/// either linking or evaluating, this is either the module's own
/// [\[\[DFSIndex\]\]](DFSIndex) or that of an "earlier" module in the same
/// strongly connected component.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DFSAncestorIndex(DFSIndex);

impl DFSAncestorIndex {
    const fn new(index: u16) -> Self {
        Self(DFSIndex::new(index))
    }

    const fn value(&self) -> u16 {
        self.0.value()
    }
}

/// ### [CyclicModuleRecord] \[\[Status\]\]
///
/// Initially new. Transitions to unlinked, linking, linked, evaluating,
/// possibly evaluating-async, evaluated (in that order) as the module
/// progresses throughout its lifecycle. evaluating-async indicates this module
/// is queued to execute on completion of its asynchronous dependencies or it
/// is a module whose \[\[HasTLA\]\] field is true that has been executed and is
/// pending top-level completion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CyclicModuleRecordStatus {
    New,
    Unlinked,
    Linking(DFSIndex, DFSAncestorIndex),
    Linked,
    Evaluating(DFSIndex, DFSAncestorIndex),
    EvaluatingAsync,
    Evaluated(Option<EvaluationError>),
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedModuleRecord {
    /// \[\[Specifier\]\]
    pub(super) specifier: Atom<'static>,
    pub(super) module: Module,
}

#[derive(Debug, Clone)]
pub(crate) struct CyclicModuleRecord {
    /// [\[\[Status\]\]](CyclicModuleRecordStatus)
    pub(super) status: CyclicModuleRecordStatus,
    /// \[\[RequestedModules\]\]
    ///
    /// A List of all the ModuleSpecifier strings used by the module
    /// represented by this record to request the importation of a module. The
    /// List is in source text occurrence order.
    pub(super) requested_modules: Box<[Atom<'static>]>,
    /// \[\[LoadedModules\]\]
    ///
    /// A map from the specifier strings used by the module represented by this
    /// record to request the importation of a module to the resolved Module
    /// Record. The list does not contain two different Records with the same
    /// \[\[Specifier\]\].
    pub(super) loaded_modules: Box<[LoadedModuleRecord]>,
    /// \[\[CycleRoot\]\]
    ///
    /// The first visited module of the cycle, the root DFS ancestor of the
    /// strongly connected component. For a module not in a cycle, this would
    /// be the module itself. Once Evaluate has completed, a module's
    /// \[\[DFSAncestorIndex\]\] is the \[\[DFSIndex\]\] of its
    /// \[\[CycleRoot\]\].
    pub(super) cycle_root: Option<Module>,
    /// \[\[HasTLA\]\]
    ///
    /// Whether this module is individually asynchronous (for example, if it's
    /// a Source Text Module Record containing a top-level await). Having an
    /// asynchronous dependency does not mean this field is true. This field
    /// must not change after the module is parsed.
    pub(super) has_top_level_await: bool,
    /// \[\[AsyncEvaluation\]\]
    ///
    /// Whether this module is either itself asynchronous or has an
    /// asynchronous dependency. Note: The order in which this field is set is
    /// used to order queued executions, see 16.2.1.5.3.4.
    pub(super) async_evaluation: bool,
    /// \[\[TopLevelCapability\]\]
    ///
    /// a PromiseCapability Record or empty
    ///
    /// If this module is the \[\[CycleRoot\]\] of some cycle, and Evaluate()
    /// was called on some module in that cycle, this field contains the
    /// PromiseCapability Record for that entire evaluation. It is used to
    /// settle the Promise object that is returned from the Evaluate() abstract
    /// method. This field will be empty for any dependencies of that module,
    /// unless a top-level Evaluate() has been initiated for some of those
    /// dependencies.
    pub(super) top_level_capability: Option<PromiseCapability>,
    /// \[\[AsyncParentModules\]\]
    ///
    /// a List of Cyclic Module Records
    ///
    /// If this module or a dependency has \[\[HasTLA\]\] true, and execution
    /// is in progress, this tracks the parent importers of this module for the
    /// top-level execution job. These parent modules will not start executing
    /// before this module has successfully completed execution.
    pub(super) async_parent_modules: Vec<Module>,
    /// \[\[PendingAsyncDependencies\]\]
    ///
    /// If this module has any asynchronous dependencies, this tracks the
    /// number of asynchronous dependency modules remaining to execute for this
    /// module. A module with asynchronous dependencies will be executed when
    /// this field reaches 0 and there are no execution errors.
    pub(super) pending_async_dependencies: Option<u16>,
}

impl CyclicModuleRecord {
    pub(crate) fn initialize_environment() {
        todo!();
    }

    pub(crate) fn execute_module(_promise_capability: Option<()>) {
        todo!();
    }
}

pub(crate) struct GraphLoadingStateRecord {
    /// \[\[PromiseCapability\]\]
    ///
    /// a PromiseCapability Record
    ///
    /// The promise to resolve when the loading process finishes.
    promise_capability: PromiseCapability,
    /// \[\[IsLoading\]\]
    ///
    /// It is true if the loading process has not finished yet, neither
    /// successfully nor with an error.
    is_loading: bool,
    /// \[\[PendingModulesCount\]\]
    ///
    /// a non-negative integer
    ///
    /// It tracks the number of pending HostLoadImportedModule calls.
    pending_modules_count: u16,
    /// \[\[Visited\]\]
    ///
    /// a List of Cyclic Module Records
    ///
    /// It is a list of the Cyclic Module Records that have been already
    /// loaded by the current loading process, to avoid infinite loops with
    /// circular dependencies.
    visited: Vec<Module>,
    /// \[\[HostDefined\]\]
    ///
    /// anything (default value is empty)
    ///
    /// It contains host-defined data to pass from the LoadRequestedModules
    /// caller to HostLoadImportedModule.
    host_defined: Option<Box<dyn Any>>,
}

impl Module {
    fn is_cyclic_module_record(self) -> bool {
        true
    }

    /// ### [16.2.1.5.1 LoadRequestedModules ( \[ hostDefined \] )](https://tc39.es/ecma262/#sec-InnerModuleLoading)
    ///
    /// The LoadRequestedModules concrete method of a Cyclic Module Record
    /// module takes optional argument hostDefined (anything) and returns a
    /// Promise. It populates the \[\[LoadedModules\]\] of all the Module
    /// Records in the dependency graph of module (most of the work is done by
    /// the auxiliary function InnerModuleLoading). It takes an optional
    /// hostDefined parameter that is passed to the HostLoadImportedModule
    /// hook.
    fn load_requested_modules(
        self,
        agent: &mut Agent,
        host_defined: Option<Box<dyn Any>>,
    ) -> Promise {
        // 1. If hostDefined is not present, let hostDefined be empty.
        // TODO: 2. Let pc be ! NewPromiseCapability(%Promise%).
        let pc = ();
        // 3. Let state be the GraphLoadingState Record {
        let mut state = GraphLoadingStateRecord {
            // [[PromiseCapability]]: pc,
            promise_capability: pc,
            // [[IsLoading]]: true,
            is_loading: true,
            // [[PendingModulesCount]]: 1,
            pending_modules_count: 1,
            // [[Visited]]: « »,
            visited: vec![],
            // [[HostDefined]]: hostDefined
            host_defined,
        };
        // }.
        // 4. Perform InnerModuleLoading(state, module).
        inner_module_loading(agent, &mut state, self);
        // 5. Return pc.[[Promise]].

        // Note
        // The hostDefined parameter can be used to pass additional information
        // necessary to fetch the imported modules. It is used, for example, by
        // HTML to set the correct fetch destination for
        // `<link rel="preload" as="...">` tags. `import()` expressions never
        // set the hostDefined parameter.
        todo!();
    }

    pub(crate) fn get_exported_names(
        self,
        agent: &mut Agent,
        export_start_set: Option<()>,
    ) -> Box<[String]> {
        todo!()
    }

    fn resolve_export(
        self,
        agent: &mut Agent,
        export_name: String,
        resolve_set: Option<()>,
    ) -> Option<ResolvedBinding> {
        todo!()
    }

    /// ### [16.2.1.5.2 Link ( )](https://tc39.es/ecma262/#sec-moduledeclarationlinking)
    ///
    /// The Link concrete method of a Cyclic Module Record module takes no
    /// arguments and returns either a normal completion containing unused or a
    /// throw completion. On success, Link transitions this module's \[\[Status\]\]
    /// from unlinked to linked. On failure, an exception is thrown and this
    /// module's \[\[Status\]\] remains unlinked. (Most of the work is done by the
    /// auxiliary function InnerModuleLinking.)
    fn link(self, agent: &mut Agent) -> JsResult<()> {
        link(agent, self)
    }

    fn evaluate(
        self,
        agent: &mut Agent,
    ) -> Result<Promise, super::abstract_module_records::NotLinkedErr> {
        Ok(evaluate(agent, self))
    }

    fn initialize_environment(self, agent: &mut Agent) {}

    fn execute_module(self, agent: &mut Agent, promise_capability: Option<()>) {}
}

/// ### [16.2.1.5.1.1 InnerModuleLoading ( state, module )](https://tc39.es/ecma262/#sec-InnerModuleLoading)
///
/// The abstract operation InnerModuleLoading takes arguments state (a GraphLoadingState Record) and module (a Module Record) and returns unused. It is used by LoadRequestedModules to recursively perform the actual loading process for module's dependency graph. It performs the following steps when called:
fn inner_module_loading(agent: &mut Agent, state: &mut GraphLoadingStateRecord, module: Module) {
    // 1. Assert: state.[[IsLoading]] is true.
    assert!(state.is_loading);
    // 2. If module is a Cyclic Module Record, module.[[Status]] is new, and
    // state.[[Visited]] does not contain module, then
    if matches!(agent[module].cyclic.status, CyclicModuleRecordStatus::New)
        && !state.visited.contains(&module)
    {
        // a. Append module to state.[[Visited]].
        state.visited.push(module);
        // b. Let requestedModulesCount be the number of elements in module.[[RequestedModules]].
        let requested_modules_count = agent[module].cyclic.requested_modules.len();
        // c. Set state.[[PendingModulesCount]] to state.[[PendingModulesCount]] + requestedModulesCount.
        state.pending_modules_count += requested_modules_count as u16;
        // d. For each String required of module.[[RequestedModules]], do
        for required in agent[module].cyclic.requested_modules.iter() {
            // i. If module.[[LoadedModules]] contains a Record whose [[Specifier]] is required, then
            let record = agent[module]
                .cyclic
                .loaded_modules
                .iter()
                .find(|record| record.specifier == required);
            // 1. Let record be that Record.
            if let Some(record) = record {
                // 2. Perform InnerModuleLoading(state, record.[[Module]]).
                inner_module_loading(agent, state, record.module);
            } else {
                // ii. Else,
                // 1. Perform HostLoadImportedModule(module, required, state.[[HostDefined]], state).
                agent.host_hooks.host_load_imported_module(
                    // agent,
                    (), // module,
                    &required,
                    state.host_defined,
                    (), // state
                );
                // 2. NOTE: HostLoadImportedModule will call FinishLoadingImportedModule,
                // which re-enters the graph loading process through ContinueModuleLoading.
            }
            // iii. If state.[[IsLoading]] is false, return unused.
            if !state.is_loading {
                return;
            }
        }
    }
    // 3. Assert: state.[[PendingModulesCount]] ≥ 1.
    assert!(state.pending_modules_count >= 1);
    // 4. Set state.[[PendingModulesCount]] to state.[[PendingModulesCount]] - 1.
    state.pending_modules_count -= 1;
    // 5. If state.[[PendingModulesCount]] = 0, then
    if state.pending_modules_count == 0 {
        // a. Set state.[[IsLoading]] to false.
        state.is_loading = false;
        // b. For each Cyclic Module Record loaded of state.[[Visited]], do
        for _loaded in state.visited {
            // TODO: i. If loaded.[[Status]] is new, set loaded.[[Status]] to unlinked.
        }
        // c. Perform ! Call(state.[[PromiseCapability]].[[Resolve]], undefined, « undefined »).
        // call_function(agent, state.promise_capability.resolve, Value::Undefined, Some(ArgumentsList(&[Value::Undefined])));
    }
    // 6. Return unused.
}

/// ### [16.2.1.5.1.2 ContinueModuleLoading ( state, moduleCompletion )](https://tc39.es/ecma262/#sec-ContinueModuleLoading)
///
/// The abstract operation ContinueModuleLoading takes arguments state (a
/// GraphLoadingState Record) and moduleCompletion (either a normal completion
/// containing a Module Record or a throw completion) and returns unused. It is
/// used to re-enter the loading process after a call to
/// HostLoadImportedModule.
fn continue_module_loading(
    agent: &mut Agent,
    state: &mut GraphLoadingStateRecord,
    module_completion: JsResult<Module>,
) {
    // 1. If state.[[IsLoading]] is false, return unused.
    if !state.is_loading {
        return;
    }
    match module_completion {
        // 2. If moduleCompletion is a normal completion, then
        // a. Perform InnerModuleLoading(state, moduleCompletion.[[Value]]).
        Ok(module) => inner_module_loading(agent, state, module),
        // 3. Else,
        Err(thrown_value) => {
            // a. Set state.[[IsLoading]] to false.
            state.is_loading = false;
            // b. Perform ! Call(state.[[PromiseCapability]].[[Reject]], undefined, « moduleCompletion.[[Value]] »).
            // call_function(state.promise_capability.reject, Value::Undefined, Some(ArgumentsList(&[thrown_value])));
        }
    }
    // 4. Return unused.
}

/// ### [16.2.1.5.2 Link ( )](https://tc39.es/ecma262/#sec-moduledeclarationlinking)
///
/// The Link concrete method of a Cyclic Module Record module takes no
/// arguments and returns either a normal completion containing unused or a
/// throw completion. On success, Link transitions this module's \[\[Status\]\]
/// from unlinked to linked. On failure, an exception is thrown and this
/// module's \[\[Status\]\] remains unlinked. (Most of the work is done by the
/// auxiliary function InnerModuleLinking.)
fn link(agent: &mut Agent, module: Module) -> JsResult<()> {
    // 1. Assert: module.[[Status]] is one of unlinked, linked, evaluating-async, or evaluated.
    assert!(matches!(
        agent[module].cyclic.status,
        CyclicModuleRecordStatus::Linked
            | CyclicModuleRecordStatus::EvaluatingAsync
            | CyclicModuleRecordStatus::Evaluated(_)
    ));
    // 2. Let stack be a new empty List.
    let mut stack = vec![];
    // 3. Let result be Completion(InnerModuleLinking(module, stack, 0)).
    let result = inner_module_linking(agent, module, &mut stack, 0);
    match result {
        // 4. If result is an abrupt completion, then
        Err(result) => {
            // a. For each Cyclic Module Record m of stack, do
            for m in stack {
                // i. Assert: m.[[Status]] is linking.
                assert!(matches!(
                    agent[m].cyclic.status,
                    CyclicModuleRecordStatus::Linking(_, _)
                ));
                // ii. Set m.[[Status]] to unlinked.
                agent[m].cyclic.status = CyclicModuleRecordStatus::Unlinked;
            }
            // b. Assert: module.[[Status]] is unlinked.
            assert_eq!(
                agent[module].cyclic.status,
                CyclicModuleRecordStatus::Unlinked
            );
            // c. Return ? result.
            return Err(result);
        }
        Ok(_) => {}
    }
    // 5. Assert: module.[[Status]] is one of linked, evaluating-async, or evaluated.
    assert!(matches!(
        agent[module].cyclic.status,
        CyclicModuleRecordStatus::Linked
            | CyclicModuleRecordStatus::EvaluatingAsync
            | CyclicModuleRecordStatus::Evaluated(_)
    ));
    // 6. Assert: stack is empty.
    assert!(stack.is_empty());
    // 7. Return unused.
    Ok(())
}

/// ### [16.2.1.5.2.1 InnerModuleLinking ( module, stack, index )](https://tc39.es/ecma262/#sec-InnerModuleLinking)
///
/// The abstract operation InnerModuleLinking takes arguments module (a Module
/// Record), stack (a List of Cyclic Module Records), and index (a non-negative
/// integer) and returns either a normal completion containing a non-negative
/// integer or a throw completion. It is used by Link to perform the actual
/// linking process for module, as well as recursively on all other modules in
/// the dependency graph. The stack and index parameters, as well as a module's
/// \[\[DFSIndex\]\] and \[\[DFSAncestorIndex\]\] fields, keep track of the
/// depth-first search (DFS) traversal. In particular, \[\[DFSAncestorIndex\]\]
/// is used to discover strongly connected components (SCCs), such that all
/// modules in an SCC transition to linked together.
fn inner_module_linking(
    agent: &mut Agent,
    module: Module,
    stack: &mut Vec<Module>,
    index: u16,
) -> JsResult<u16> {
    // 1. If module is not a Cyclic Module Record, then
    if !module.is_cyclic_module_record() {
        // a. Perform ? module.Link().
        module.link(agent)?;
        // b. Return index.
        return Ok(index);
    }
    // 2. If module.[[Status]] is one of linking, linked, evaluating-async, or
    // evaluated, then
    match agent[module].cyclic.status {
        CyclicModuleRecordStatus::Linking(_, _)
        | CyclicModuleRecordStatus::Linked
        | CyclicModuleRecordStatus::EvaluatingAsync => {
            // a. Return index.
            return Ok(index);
        }
        _ => {}
    }
    // 3. Assert: module.[[Status]] is unlinked.
    assert_eq!(
        agent[module].cyclic.status,
        CyclicModuleRecordStatus::Unlinked
    );
    // 4. Set module.[[Status]] to linking.
    // 5. Set module.[[DFSIndex]] to index.
    // 6. Set module.[[DFSAncestorIndex]] to index.
    agent[module].cyclic.status =
        CyclicModuleRecordStatus::Linking(DFSIndex::new(index), DFSAncestorIndex::new(index));
    // 7. Set index to index + 1.
    let mut index = index + 1;
    // 8. Append module to stack.
    stack.push(module);
    // 9. For each String required of module.[[RequestedModules]], do
    for required in agent[module].cyclic.requested_modules.iter() {
        // a. Let requiredModule be GetImportedModule(module, required).
        let required_module = get_imported_module(agent, module, *required);
        // b. Set index to ? InnerModuleLinking(requiredModule, stack, index).
        index = inner_module_linking(agent, required_module, stack, index)?;
        // c. If requiredModule is a Cyclic Module Record, then
        if required_module.is_cyclic_module_record() {
            // i. Assert: requiredModule.[[Status]] is one of linking, linked, evaluating-async, or evaluated.
            assert!(matches!(
                agent[required_module].cyclic.status,
                CyclicModuleRecordStatus::Linked
                    | CyclicModuleRecordStatus::EvaluatingAsync
                    | CyclicModuleRecordStatus::Evaluated(_)
            ));
            // ii. Assert: requiredModule.[[Status]] is linking if and only if stack contains requiredModule.
            assert_eq!(
                matches!(
                    agent[required_module].cyclic.status,
                    CyclicModuleRecordStatus::Linking(_, _)
                ),
                stack.contains(&required_module)
            );
            // iii. If requiredModule.[[Status]] is linking, then
            if let CyclicModuleRecordStatus::Linking(_, ancestor_index) =
                agent[required_module].cyclic.status
            {
                assert!(matches!(
                    agent[module].cyclic.status,
                    CyclicModuleRecordStatus::Evaluating(_, _)
                ));
                // 1. Set module.[[DFSAncestorIndex]] to min(module.[[DFSAncestorIndex]], requiredModule.[[DFSAncestorIndex]]).
                match &mut agent[module].cyclic.status {
                    CyclicModuleRecordStatus::Evaluating(_, module_ancestor_index) => {
                        let min = module_ancestor_index.value().min(ancestor_index.value());
                        *module_ancestor_index = DFSAncestorIndex::new(min);
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    // 10. Perform ? module.InitializeEnvironment().
    module.initialize_environment(agent);
    // 11. Assert: module occurs exactly once in stack.
    assert_eq!(
        stack
            .iter()
            .filter(|entry| **entry == module)
            .count(),
        1
    );
    // 12. Assert: module.[[DFSAncestorIndex]] ≤ module.[[DFSIndex]].
    match &mut agent[module].cyclic.status {
        CyclicModuleRecordStatus::Evaluating(index, ancestor_index) => {
            assert!(ancestor_index.value() <= index.value());
            // 13. If module.[[DFSAncestorIndex]] = module.[[DFSIndex]], then
            if ancestor_index.value() == index.value() {
                // a. Let done be false.
                let mut done = false;
                // b. Repeat, while done is false,
                while !done {
                    // i. Let requiredModule be the last element of stack.
                    // ii. Remove the last element of stack.
                    let required_module = stack.pop().unwrap();
                    // iii. Assert: requiredModule is a Cyclic Module Record.
                    assert!(required_module.is_cyclic_module_record());
                    // iv. Set requiredModule.[[Status]] to linked.
                    agent[required_module].cyclic.status = CyclicModuleRecordStatus::Linked;
                    // v. If requiredModule and module are the same Module Record, set done to true.
                    if required_module == module {
                        done = true;
                    }
                }
            }
        }
        _ => unreachable!(),
    }
    // 14. Return index.
    Ok(index)
}

/// ### [16.2.1.5.3 Evaluate ( )]()
///
/// The Evaluate concrete method of a Cyclic Module Record module takes no
/// arguments and returns a Promise. Evaluate transitions this module's
/// \[\[Status\]\] from linked to either evaluating-async or evaluated. The
/// first time it is called on a module in a given strongly connected
/// component, Evaluate creates and returns a Promise which resolves when the
/// module has finished evaluating. This Promise is stored in the
/// \[\[TopLevelCapability\]\] field of the \[\[CycleRoot\]\] for the
/// component. Future invocations of Evaluate on any module in the component
/// return the same Promise. (Most of the work is done by the auxiliary
/// function InnerModuleEvaluation.)
pub(crate) fn evaluate(agent: &mut Agent, mut module: Module) -> Promise {
    // 1. Assert: This call to Evaluate is not happening at the same time as another call to Evaluate within the surrounding agent.
    // TODO: How to figure this one out?
    // 2. Assert: module.[[Status]] is one of linked, evaluating-async, or evaluated.
    assert!(matches!(
        agent[module].cyclic.status,
        CyclicModuleRecordStatus::Linked
            | CyclicModuleRecordStatus::EvaluatingAsync
            | CyclicModuleRecordStatus::Evaluated(_)
    ));
    match agent[module].cyclic.status {
        CyclicModuleRecordStatus::Linked => {}
        // 3. If module.[[Status]] is either evaluating-async or evaluated, set module to module.[[CycleRoot]].
        CyclicModuleRecordStatus::EvaluatingAsync | CyclicModuleRecordStatus::Evaluated(_) => {
            module = agent[module].cyclic.cycle_root.unwrap();
        }
        _ => unreachable!(),
    }
    // 4. If module.[[TopLevelCapability]] is not empty, then
    if let Some(tlc) = agent[module].cyclic.top_level_capability {
        // a. Return module.[[TopLevelCapability]].[[Promise]].
        todo!();
    }
    // 5. Let stack be a new empty List.
    let mut stack = vec![];
    // 6. Let capability be ! NewPromiseCapability(%Promise%).
    let capability = (); // new_promise_capability();
                         // 7. Set module.[[TopLevelCapability]] to capability.
    agent[module].cyclic.top_level_capability = Some(capability);
    // 8. Let result be Completion(InnerModuleEvaluation(module, stack, 0)).
    let result = inner_module_evaluation(agent, module, &mut stack, 0);
    // 9. If result is an abrupt completion, then
    if result.is_err() {
        let result_value = result.err().unwrap();
        // a. For each Cyclic Module Record m of stack, do
        for m in stack {
            // i. Assert: m.[[Status]] is evaluating.
            assert!(matches!(
                agent[m].cyclic.status,
                CyclicModuleRecordStatus::Evaluating(_, _)
            ));
            // ii. Set m.[[Status]] to evaluated.
            // iii. Set m.[[EvaluationError]] to result.
            agent[m].cyclic.status =
                CyclicModuleRecordStatus::Evaluated(Some(EvaluationError(result_value)));
        }
        // b. Assert: module.[[Status]] is evaluated.
        // c. Assert: module.[[EvaluationError]] and result are the same Completion Record.
        assert_eq!(
            agent[module].cyclic.status,
            CyclicModuleRecordStatus::Evaluated(Some(EvaluationError(result_value)))
        );
        // d. Perform ! Call(capability.[[Reject]], undefined, « result.[[Value]] »).
        // call_function(agent, capability.reject, Value::Undefined, Some(ArgumentsList(&[result_value.0]))).unwrap();
    } else {
        // 10. Else,
        // a. Assert: module.[[Status]] is either evaluating-async or evaluated.
        // b. Assert: module.[[EvaluationError]] is empty.
        assert!(matches!(
            agent[module].cyclic.status,
            CyclicModuleRecordStatus::EvaluatingAsync | CyclicModuleRecordStatus::Evaluated(None)
        ));
        // c. If module.[[AsyncEvaluation]] is false, then
        if !agent[module].cyclic.async_evaluation {
            // i. Assert: module.[[Status]] is evaluated.
            assert_eq!(
                agent[module].cyclic.status,
                CyclicModuleRecordStatus::Evaluated(None)
            );
        }
        // ii. Perform ! Call(capability.[[Resolve]], undefined, « undefined »).
        // call_function(agent, capability.resolve, Value::Undefined, Some(ArgumentsList(&[Value::Undefined])));
        // d. Assert: stack is empty.
        assert!(stack.is_empty());
    }
    // 11. Return capability.[[Promise]].
    todo!();
}

/// ### [16.2.1.5.3.1 InnerModuleEvaluation ( module, stack, index )]()
///
/// The abstract operation InnerModuleEvaluation takes arguments module (a
/// Module Record), stack (a List of Cyclic Module Records), and index (a
/// non-negative integer) and returns either a normal completion containing a
/// non-negative integer or a throw completion. It is used by Evaluate to
/// perform the actual evaluation process for module, as well as recursively on
/// all other modules in the dependency graph. The stack and index parameters,
/// as well as module's \[\[DFSIndex\]\] and \[\[DFSAncestorIndex\]\] fields,
/// are used the same way as in InnerModuleLinking.
pub(crate) fn inner_module_evaluation(
    agent: &mut Agent,
    module: Module,
    stack: &mut Vec<Module>,
    index: u16,
) -> JsResult<u16> {
    // 1. If module is not a Cyclic Module Record, then
    if module.is_cyclic_module_record() {
        // a. Let promise be ! module.Evaluate().
        let promise = module.evaluate(agent).unwrap();
        // b. Assert: promise.[[PromiseState]] is not pending.
        // c. If promise.[[PromiseState]] is rejected, then
        let is_rejected = false;
        if is_rejected {
            // i. Return ThrowCompletion(promise.[[PromiseResult]]).
            return Err(JsError(Value::Undefined));
        }
        // d. Return index.
        return Ok(index);
    }
    let module_borrow = &agent[module];
    // 2. If module.[[Status]] is either evaluating-async or evaluated, then
    match module_borrow.cyclic.status {
        // a. If module.[[EvaluationError]] is empty, return index.
        // b. Otherwise, return ? module.[[EvaluationError]].
        CyclicModuleRecordStatus::EvaluatingAsync => {
            return Ok(index);
        }
        CyclicModuleRecordStatus::Evaluated(maybe_evaluation_error) => match maybe_evaluation_error
        {
            Some(error) => Err(error.0),
            None => {
                return Ok(index);
            }
        },
        CyclicModuleRecordStatus::Evaluating(_, _) => {
            // 3. If module.[[Status]] is evaluating, return index.
            Ok(index)
        }
        CyclicModuleRecordStatus::Linked => {
            // 5. Set module.[[Status]] to evaluating.
            // 6. Set module.[[DFSIndex]] to index.
            // 7. Set module.[[DFSAncestorIndex]] to index.
            module_borrow.cyclic.status = CyclicModuleRecordStatus::Evaluating(
                DFSIndex::new(index),
                DFSAncestorIndex::new(index),
            );
            // 8. Set module.[[PendingAsyncDependencies]] to 0.
            module_borrow.cyclic.pending_async_dependencies = Some(0);
            // 9. Set index to index + 1.
            let mut index = index + 1;
            // 10. Append module to stack.
            stack.push(module);
            // 11. For each String required of module.[[RequestedModules]], do
            for required in module_borrow.cyclic.requested_modules.iter() {
                // a. Let requiredModule be GetImportedModule(module, required).
                let mut required_module = get_imported_module(agent, module, *required);
                // b. Set index to ? InnerModuleEvaluation(requiredModule, stack, index).
                index = inner_module_evaluation(agent, required_module, stack, index)?;
                // c. If requiredModule is a Cyclic Module Record, then
                if required_module.is_cyclic_module_record() {
                    let required_module_borrow = &agent[required_module];
                    let stack_contains_required_module = stack.contains(&required_module);
                    match required_module_borrow.cyclic.status {
                        CyclicModuleRecordStatus::Evaluating(_, required_module_ancestor_index) => {
                            // ii. Assert: requiredModule.[[Status]] is evaluating if and only if stack contains requiredModule.
                            assert!(stack_contains_required_module);
                            // iii. If requiredModule.[[Status]] is evaluating, then
                            // 1. Set module.[[DFSAncestorIndex]] to min(module.[[DFSAncestorIndex]], requiredModule.[[DFSAncestorIndex]]).
                            let module_borrow = &mut agent[module];
                            match &mut module_borrow.cyclic.status {
                                CyclicModuleRecordStatus::Evaluating(_, ancestor_index) => {
                                    *ancestor_index = DFSAncestorIndex::new(
                                        ancestor_index
                                            .value()
                                            .min(required_module_ancestor_index.value()),
                                    );
                                }
                                _ => unreachable!(),
                            }
                        }
                        // iv. Else,
                        CyclicModuleRecordStatus::EvaluatingAsync => {
                            assert!(!stack_contains_required_module);
                            // 1. Set requiredModule to requiredModule.[[CycleRoot]].
                            required_module = required_module_borrow.cyclic.cycle_root.unwrap();
                            let required_module_borrow = &agent[required_module];
                            match required_module_borrow.cyclic.status {
                                CyclicModuleRecordStatus::EvaluatingAsync => {
                                    // v. If requiredModule.[[AsyncEvaluation]] is true, then
                                    if required_module_borrow.cyclic.async_evaluation {
                                        // 1. Set module.[[PendingAsyncDependencies]] to module.[[PendingAsyncDependencies]] + 1.
                                        agent[module]
                                            .cyclic
                                            .pending_async_dependencies
                                            .as_mut()
                                            .map(|val| *val += 1);
                                        // 2. Append module to requiredModule.[[AsyncParentModules]].
                                        required_module_borrow
                                            .cyclic
                                            .async_parent_modules
                                            .push(module);
                                    }
                                }
                                CyclicModuleRecordStatus::Evaluated(maybe_evaluation_error) => {
                                    // 3. If requiredModule.[[EvaluationError]] is not empty,
                                    if let Some(evaluation_error) = maybe_evaluation_error {
                                        // return ? requiredModule.[[EvaluationError]].
                                        return Err(evaluation_error.0);
                                    }
                                    // v. If requiredModule.[[AsyncEvaluation]] is true, then
                                    if required_module_borrow.cyclic.async_evaluation {
                                        // 1. Set module.[[PendingAsyncDependencies]] to module.[[PendingAsyncDependencies]] + 1.
                                        agent[module]
                                            .cyclic
                                            .pending_async_dependencies
                                            .as_mut()
                                            .map(|val| *val += 1);
                                        // 2. Append module to requiredModule.[[AsyncParentModules]].
                                        required_module_borrow
                                            .cyclic
                                            .async_parent_modules
                                            .push(module);
                                    }
                                }
                                // 2. Assert: requiredModule.[[Status]] is either evaluating-async or evaluated.
                                _ => unreachable!(),
                            }
                        }
                        CyclicModuleRecordStatus::Evaluated(_) => {
                            assert!(!stack_contains_required_module);
                            // 1. Set requiredModule to requiredModule.[[CycleRoot]].
                            required_module = required_module_borrow.cyclic.cycle_root.unwrap();
                            let required_module_borrow = &agent[required_module];
                            match required_module_borrow.cyclic.status {
                                CyclicModuleRecordStatus::EvaluatingAsync => {
                                    // v. If requiredModule.[[AsyncEvaluation]] is true, then
                                    if required_module_borrow.cyclic.async_evaluation {
                                        // 1. Set module.[[PendingAsyncDependencies]] to module.[[PendingAsyncDependencies]] + 1.
                                        agent[module]
                                            .cyclic
                                            .pending_async_dependencies
                                            .as_mut()
                                            .map(|val| *val += 1);
                                        // 2. Append module to requiredModule.[[AsyncParentModules]].
                                        required_module_borrow
                                            .cyclic
                                            .async_parent_modules
                                            .push(module);
                                    }
                                }
                                CyclicModuleRecordStatus::Evaluated(maybe_evaluation_error) => {
                                    // 3. If requiredModule.[[EvaluationError]] is not empty,
                                    if let Some(evaluation_error) = maybe_evaluation_error {
                                        // return ? requiredModule.[[EvaluationError]].
                                        return Err(evaluation_error.0);
                                    }
                                    // v. If requiredModule.[[AsyncEvaluation]] is true, then
                                    if required_module_borrow.cyclic.async_evaluation {
                                        // 1. Set module.[[PendingAsyncDependencies]] to module.[[PendingAsyncDependencies]] + 1.
                                        agent[module]
                                            .cyclic
                                            .pending_async_dependencies
                                            .as_mut()
                                            .map(|val| *val += 1);
                                        // 2. Append module to requiredModule.[[AsyncParentModules]].
                                        required_module_borrow
                                            .cyclic
                                            .async_parent_modules
                                            .push(module);
                                    }
                                }
                                // 2. Assert: requiredModule.[[Status]] is either evaluating-async or evaluated.
                                _ => unreachable!(),
                            }
                        }
                        // i. Assert: requiredModule.[[Status]] is one of evaluating, evaluating-async, or evaluated.
                        _ => unreachable!(),
                    }
                }
            }
            // 12. If module.[[PendingAsyncDependencies]] > 0 or module.[[HasTLA]] is true, then
            if agent[module].cyclic.pending_async_dependencies.unwrap() > 0
                || agent[module].cyclic.has_top_level_await
            {
                // a. Assert: module.[[AsyncEvaluation]] is false and was never previously set to true.
                assert!(agent[module].cyclic.async_evaluation);
                // b. Set module.[[AsyncEvaluation]] to true.
                agent[module].cyclic.async_evaluation = true;
                // c. NOTE: The order in which module records have their
                // [[AsyncEvaluation]] fields transition to true is
                // significant. (See 16.2.1.5.3.4.)
                // d. If module.[[PendingAsyncDependencies]] = 0,
                if agent[module].cyclic.pending_async_dependencies == Some(0) {
                    // perform ExecuteAsyncModule(module).
                    execute_async_module(agent, module);
                }
            } else {
                // 13. Else,
                // a. Perform ? module.ExecuteModule().
                module.execute_module(agent, None);
            }
            // 14. Assert: module occurs exactly once in stack.
            assert_eq!(
                stack
                    .iter()
                    .filter(|entry| **entry == module)
                    .count(),
                1
            );
            let module_borrow = &agent[module];
            match module_borrow.cyclic.status {
                CyclicModuleRecordStatus::Evaluating(index, ancestor_index) => {
                    // 15. Assert: module.[[DFSAncestorIndex]] ≤ module.[[DFSIndex]].
                    assert!(ancestor_index.value() <= index.value());
                    // 16. If module.[[DFSAncestorIndex]] = module.[[DFSIndex]], then
                    if ancestor_index.value() == index.value() {
                        // a. Let done be false.
                        let mut done = false;
                        while !done {
                            // b. Repeat, while done is false,
                            // i. Let requiredModule be the last element of stack.
                            // ii. Remove the last element of stack.
                            let required_module = stack.pop().unwrap();
                            // iii. Assert: requiredModule is a Cyclic Module Record.
                            assert!(required_module.is_cyclic_module_record());
                            // iv. If requiredModule.[[AsyncEvaluation]] is false, set requiredModule.[[Status]] to evaluated.
                            if !agent[required_module].cyclic.async_evaluation {
                                agent[required_module].cyclic.status =
                                    CyclicModuleRecordStatus::Evaluated(None);
                            } else {
                                // v. Otherwise, set requiredModule.[[Status]] to evaluating-async.
                                agent[required_module].cyclic.status =
                                    CyclicModuleRecordStatus::EvaluatingAsync;
                            }
                            // vi. If requiredModule and module are the same Module Record, set done to true.
                            if required_module == module {
                                done = true;
                            }
                            // vii. Set requiredModule.[[CycleRoot]] to module.
                            agent[required_module].cyclic.cycle_root = Some(module);
                        }
                    }
                }
                _ => unreachable!(),
            }
            // 17. Return index.
            Ok(index)
        }
        // 4. Assert: module.[[Status]] is linked.
        _ => unreachable!(),
    }

    // Note 1

    // A module is evaluating while it is being traversed by
    // InnerModuleEvaluation. A module is evaluated on execution completion or
    // evaluating-async during execution if its [[HasTLA]] field is true or if
    // it has asynchronous dependencies.
    // Note 2

    // Any modules depending on a module of an asynchronous cycle when that
    // cycle is not evaluating will instead depend on the execution of the root
    // of the cycle via [[CycleRoot]]. This ensures that the cycle state can be
    // treated as a single strongly connected component through its root module
    // state.
}

/// ### [16.2.1.5.3.2 ExecuteAsyncModule ( module )]()
///
/// The abstract operation ExecuteAsyncModule takes argument module (a Cyclic
/// Module Record) and returns unused.
pub(crate) fn execute_async_module(agent: &mut Agent, module: Module) {
    let module_borrow = &agent[module];
    // 1. Assert: module.[[Status]] is either evaluating or evaluating-async.
    assert!(matches!(
        module_borrow.cyclic.status,
        CyclicModuleRecordStatus::Evaluating(_, _) | CyclicModuleRecordStatus::EvaluatingAsync
    ));
    // 2. Assert: module.[[HasTLA]] is true.
    assert!(module_borrow.cyclic.has_top_level_await);
    // 3. Let capability be ! NewPromiseCapability(%Promise%).
    let capability = (); // new_promise_capability(agent, ProtoIntrinsics::Promise);
                         // 4. Let fulfilledClosure be a new Abstract Closure with no parameters that captures module and performs the following steps when called:
                         // a. Perform AsyncModuleExecutionFulfilled(module).
                         // b. Return undefined.
                         // 5. Let onFulfilled be CreateBuiltinFunction(fulfilledClosure, 0, "", « »).
    let on_fulfilled = ();
    // 6. Let rejectedClosure be a new Abstract Closure with parameters (error) that captures module and performs the following steps when called:
    // a. Perform AsyncModuleExecutionRejected(module, error).
    // b. Return undefined.
    // 7. Let onRejected be CreateBuiltinFunction(rejectedClosure, 0, "", « »).
    let on_rejected = ();
    // 8. Perform PerformPromiseThen(capability.[[Promise]], onFulfilled, onRejected).
    perform_promise_then(capability.promise, on_fulfilled, on_rejected);
    // 9. Perform ! module.ExecuteModule(capability).
    module.execute_module(agent, Some(capability));
    // 10. Return unused.
}

fn fulfilled_closure(
    agent: &mut Agent,
    this_value: Value,
    arguments: Option<ArgumentsList>,
    module: Module,
) -> JsResult<Value> {
    // a. Perform AsyncModuleExecutionFulfilled(module).
    async_module_execution_fulfilled(agent, module);
    // b. Return undefined.
    Ok(Value::Undefined)
}

fn rejected_closure(
    agent: &mut Agent,
    this_value: Value,
    arguments: Option<ArgumentsList>,
    module: Module,
) -> JsResult<Value> {
    async_module_execution_rejected(agent, module, arguments.unwrap().get(0));
    Ok(Value::Undefined)
}

/// ### [16.2.1.5.3.3 GatherAvailableAncestors ( module, execList )]()
///
/// The abstract operation GatherAvailableAncestors takes arguments module (a
/// Cyclic Module Record) and execList (a List of Cyclic Module Records) and
/// returns unused.
pub(crate) fn gather_available_ancestors(
    agent: &mut Agent,
    module: Module,
    exec_list: &mut Vec<Module>,
) {
    // 1. For each Cyclic Module Record m of module.[[AsyncParentModules]], do
    for m in agent[module].cyclic.async_parent_modules {
        // a. If execList does not contain m and m.[[CycleRoot]].[[EvaluationError]] is empty, then
        if !exec_list.contains(&m)
            && !matches!(
                agent[agent[m].cyclic.cycle_root.unwrap()].cyclic.status,
                CyclicModuleRecordStatus::Evaluated(Some(_))
            )
        {
            // i. Assert: m.[[Status]] is evaluating-async.
            // ii. Assert: m.[[EvaluationError]] is empty.
            assert!(matches!(
                agent[m].cyclic.status,
                CyclicModuleRecordStatus::EvaluatingAsync
            ));
            // iii. Assert: m.[[AsyncEvaluation]] is true.
            assert!(agent[m].cyclic.async_evaluation);
            // iv. Assert: m.[[PendingAsyncDependencies]] > 0.
            assert!(agent[m].cyclic.pending_async_dependencies.unwrap() > 0);
            // v. Set m.[[PendingAsyncDependencies]] to m.[[PendingAsyncDependencies]] - 1.
            agent[m]
                .cyclic
                .pending_async_dependencies
                .as_mut()
                .map(|val| *val -= 1);
            // vi. If m.[[PendingAsyncDependencies]] = 0, then
            if agent[m].cyclic.pending_async_dependencies == Some(0) {
                // 1. Append m to execList.
                exec_list.push(m);
            }
        }
        // 2. If m.[[HasTLA]] is false, perform GatherAvailableAncestors(m, execList).
        if !agent[m].cyclic.has_top_level_await {
            gather_available_ancestors(agent, m, exec_list);
        }
    }
    // 2. Return unused.

    // Note

    // When an asynchronous execution for a root module is fulfilled, this
    // function determines the list of modules which are able to synchronously
    // execute together on this completion, populating them in execList.
}

/// ### [16.2.1.5.3.4 AsyncModuleExecutionFulfilled ( module )]()
///
/// The abstract operation AsyncModuleExecutionFulfilled takes argument module
/// // (a Cyclic Module Record) and returns unused.
pub(crate) fn async_module_execution_fulfilled(agent: &mut Agent, module: Module) {
    let module_borrow = &agent[module].cyclic;
    // 1. If module.[[Status]] is evaluated, then
    match module_borrow.status {
        CyclicModuleRecordStatus::Evaluated(maybe_evaluation_error) => {
            // a. Assert: module.[[EvaluationError]] is not empty.
            assert!(maybe_evaluation_error.is_none());
            // b. Return unused.
            return;
        }
        // 2. Assert: module.[[Status]] is evaluating-async.
        CyclicModuleRecordStatus::EvaluatingAsync => {}
        _ => unreachable!(),
    }
    // 3. Assert: module.[[AsyncEvaluation]] is true.
    assert!(module_borrow.async_evaluation);
    // 4. Assert: module.[[EvaluationError]] is empty.
    // 5. Set module.[[AsyncEvaluation]] to false.
    module_borrow.async_evaluation = false;
    // 6. Set module.[[Status]] to evaluated.
    module_borrow.status = CyclicModuleRecordStatus::Evaluated(None);
    // 7. If module.[[TopLevelCapability]] is not empty, then
    if module_borrow.top_level_capability.is_some() {
        // a. Assert: module.[[CycleRoot]] and module are the same Module Record.
        assert_eq!(module_borrow.cycle_root.unwrap(), module);
        // b. Perform ! Call(module.[[TopLevelCapability]].[[Resolve]], undefined, « undefined »).
        // call_function(agent, module_borrow.top_level_capability.unwrap().resolve, Value::Undefined, Some(ArgumentsList(&[Value::Undefined]))).unwrap();
    }
    // 8. Let execList be a new empty List.
    let mut exec_list = Vec::with_capacity(module_borrow.async_parent_modules.len());
    // 9. Perform GatherAvailableAncestors(module, execList).
    gather_available_ancestors(agent, module, &mut exec_list);
    // 10. Let sortedExecList be a List whose elements are the elements of
    // execList, in the order in which they had their [[AsyncEvaluation]]
    // fields set to true in InnerModuleEvaluation.
    // TODO: exec_list.sort();
    // 11. Assert: All elements of sortedExecList have their
    // [[AsyncEvaluation]] field set to true, [[PendingAsyncDependencies]]
    // field set to 0, and [[EvaluationError]] field set to empty.
    for element in exec_list {
        assert!(agent[element].cyclic.async_evaluation);
        assert_eq!(agent[element].cyclic.pending_async_dependencies, Some(0));
        assert!(!matches!(
            agent[element].cyclic.status,
            CyclicModuleRecordStatus::Evaluated(Some(_))
        ));
    }
    // 12. For each Cyclic Module Record m of sortedExecList, do
    for m in exec_list {
        // a. If m.[[Status]] is evaluated, then
        if let CyclicModuleRecordStatus::Evaluated(maybe_evaluation_error) = agent[m].cyclic.status
        {
            // i. Assert: m.[[EvaluationError]] is not empty.
            assert!(maybe_evaluation_error.is_none());
        } else if agent[m].cyclic.has_top_level_await {
            // b. Else if m.[[HasTLA]] is true, then
            // i. Perform ExecuteAsyncModule(m).
            execute_async_module(agent, m);
        } else {
            // c. Else,
            // i. Let result be m.ExecuteModule().
            let result = m.execute_module(agent, None);
            match result {
                // ii. If result is an abrupt completion, then
                Err(error) => {
                    // 1. Perform AsyncModuleExecutionRejected(m, result.[[Value]]).
                    async_module_execution_rejected(agent, m, error);
                }
                // iii. Else,
                Ok(_) => {
                    // 1. Set m.[[Status]] to evaluated.
                    agent[m].cyclic.status = CyclicModuleRecordStatus::Evaluated(None);
                    // 2. If m.[[TopLevelCapability]] is not empty, then
                    if agent[m].cyclic.top_level_capability.is_some() {
                        // a. Assert: m.[[CycleRoot]] and m are the same Module Record.
                        assert_eq!(agent[m].cyclic.cycle_root.unwrap(), m);
                        // b. Perform ! Call(m.[[TopLevelCapability]].[[Resolve]], undefined, « undefined »).
                        // call_function(agent[m].module.top_level_capability.unwrap().resolve, Value::Undefined, Some(ArgumentsList(&[Value::Undefined])));
                    }
                }
            }
        }
    }
    // 13. Return unused.
}

/// ### [16.2.1.5.3.5 AsyncModuleExecutionRejected ( module, error )]()
///
/// The abstract operation AsyncModuleExecutionRejected takes arguments module
/// (a Cyclic Module Record) and error (an ECMAScript language value) and
/// returns unused.
pub(crate) fn async_module_execution_rejected(agent: &mut Agent, module: Module, error: Value) {
    // 1. If module.[[Status]] is evaluated, then
    if let CyclicModuleRecordStatus::Evaluated(maybe_evaluation_error) = agent[module].cyclic.status
    {
        // a. Assert: module.[[EvaluationError]] is not empty.
        assert!(maybe_evaluation_error.is_some());
        // b. Return unused.
        return;
    }
    // 2. Assert: module.[[Status]] is evaluating-async.
    // 4. Assert: module.[[EvaluationError]] is empty.
    assert_eq!(
        agent[module].cyclic.status,
        CyclicModuleRecordStatus::EvaluatingAsync
    );
    // 3. Assert: module.[[AsyncEvaluation]] is true.
    assert!(agent[module].cyclic.async_evaluation);
    // 5. Set module.[[EvaluationError]] to ThrowCompletion(error).
    // 6. Set module.[[Status]] to evaluated.
    agent[module].cyclic.status =
        CyclicModuleRecordStatus::Evaluated(Some(EvaluationError(JsError(error))));
    // 7. For each Cyclic Module Record m of module.[[AsyncParentModules]], do
    for m in agent[module].cyclic.async_parent_modules {
        // a. Perform AsyncModuleExecutionRejected(m, error).
        async_module_execution_rejected(agent, m, error);
    }
    // 8. If module.[[TopLevelCapability]] is not empty, then
    if agent[module].cyclic.top_level_capability.is_some() {
        // a. Assert: module.[[CycleRoot]] and module are the same Module Record.
        assert_eq!(agent[module].cyclic.cycle_root.unwrap(), module);
        // b. Perform ! Call(module.[[TopLevelCapability]].[[Reject]], undefined, « error »).
        // call_function(agent, agent[module].module.top_level_capability.unwrap().reject, Value::Undefined, Some(ArgumentsList(&[Value::Undefined])));
    }
    // 9. Return unused.
}
