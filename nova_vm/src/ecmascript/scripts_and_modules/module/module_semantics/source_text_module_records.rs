//! ## [16.2.1.7 Source Text Module Records](https://tc39.es/ecma262/#sec-source-text-module-records)

use std::{marker::PhantomData, mem::ManuallyDrop};

use ahash::AHashSet;
use oxc_ast::ast::{self, Program};
use oxc_diagnostics::OxcDiagnostic;
use oxc_ecmascript::BoundNames;
use oxc_span::SourceType;

use crate::{
    ecmascript::{
        builtins::{
            promise::Promise,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{
            Agent, ECMAScriptCodeEvaluationState, ExecutionContext, JsResult, ModuleEnvironment,
            Realm, agent::JsError, new_module_environment,
        },
        scripts_and_modules::{
            ScriptOrModule,
            module::module_semantics::cyclic_module_records::{
                CyclicModuleRecordStatus, inner_module_evaluation, inner_module_linking,
            },
            script::HostDefined,
            source_code::SourceCode,
        },
        syntax_directed_operations::{
            miscellaneous::instantiate_function_object,
            scope_analysis::{
                LexicallyScopedDeclaration, VarScopedDeclaration,
                module_lexically_scoped_declarations, module_var_scoped_declarations,
            },
        },
        types::{IntoValue, Object, String, Value},
    },
    engine::{
        Executable, Vm,
        context::{Bindable, GcScope, GcToken, NoGcScope},
        rootable::Scopable,
    },
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};

use super::{
    abstract_module_records::{AbstractModuleRecord, ModuleAbstractMethods},
    cyclic_module_records::{CyclicModuleAbstractMethods, CyclicModuleRecord},
};

#[derive(Debug)]
/// ### [16.2.1.7 Source Text Module Records](https://tc39.es/ecma262/#sec-source-text-module-records)
pub(crate) struct SourceTextModuleRecord<'a> {
    abstract_fields: AbstractModuleRecord<'a>,
    cyclic_fields: CyclicModuleRecord<'a>,
    /// ### \[\[ECMAScriptCode]]
    ///
    /// a Parse Node
    ///
    /// The result of parsing the source text of this module using Module as the goal symbol.
    ///
    /// Note: The Program's drop code is never run. The referred structures
    /// live in the SourceCode heap data in its contained Allocator. The bump
    /// allocator drops all of the data in a single go. All that needs to be
    /// dropped here is the local Program itself, not any of its referred
    /// parts.
    ecmascript_code: ManuallyDrop<Program<'static>>,
    /// ### \[\[Context]]
    ///
    /// an ECMAScript code execution context or empty
    ///
    /// The execution context associated with this module. It is empty until
    /// the module's environment has been initialized.
    context: (),
    /// ### \[\[ImportMeta]]
    ///
    /// an Object or empty
    ///
    /// An object exposed through the import.meta meta property. It is empty
    /// until it is accessed by ECMAScript code.
    import_meta: Option<Object<'a>>,
    /// ### \[\[ImportEntries]]
    ///
    /// a List of ImportEntry Records
    ///
    /// A List of ImportEntry records derived from the code of this module.
    import_entries: (),
    /// ### \[\[LocalExportEntries]]
    ///
    /// a List of ExportEntry Records
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to declarations that occur within the module.
    local_export_entries: (),
    /// ### \[\[IndirectExportEntries]]
    ///
    /// a List of ExportEntry Records
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to reexported imports that occur within the module or
    /// exports from export * as namespace declarations.
    indirect_export_entries: (),
    /// ### \[\[StarExportEntries]]
    ///
    /// a List of ExportEntry Records
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to export * declarations that occur within the module, not
    /// including export * as namespace declarations.
    star_export_entries: (),

    /// Source text of the script
    ///
    /// The source text is kept in the heap strings vector, through the
    /// SourceCode struct.
    pub(crate) source_code: SourceCode<'a>,
}

impl<'a> SourceTextModuleRecord<'a> {
    fn new(
        realm: Realm<'a>,
        host_defined: Option<HostDefined>,
        r#async: bool,
        requested_modules: (),
        body: Program<'static>,
        source_code: SourceCode<'a>,
    ) -> Self {
        // 12. Return Source Text Module Record {
        Self {
            // [[Realm]]: realm,
            // [[Environment]]: empty,
            // [[Namespace]]: empty,
            // [[HostDefined]]: hostDefined,
            abstract_fields: AbstractModuleRecord::new(realm, host_defined),
            // [[CycleRoot]]: empty,
            // [[HasTLA]]: async,
            // [[AsyncEvaluationOrder]]: unset,
            // [[TopLevelCapability]]: empty,
            // [[AsyncParentModules]]: « »,
            // [[PendingAsyncDependencies]]: empty,
            // [[Status]]: new,
            // [[EvaluationError]]: empty,
            // [[RequestedModules]]: requestedModules,
            // [[LoadedModules]]: « »,
            // [[DFSIndex]]: empty,
            // [[DFSAncestorIndex]]: empty
            cyclic_fields: CyclicModuleRecord::new(r#async, requested_modules),
            // [[ECMAScriptCode]]: body,
            ecmascript_code: ManuallyDrop::new(body),
            // [[Context]]: empty,
            context: Default::default(),
            // [[ImportMeta]]: empty,
            import_meta: Default::default(),
            // [[ImportEntries]]: importEntries,
            import_entries: Default::default(),
            // [[LocalExportEntries]]: localExportEntries,
            local_export_entries: Default::default(),
            // [[IndirectExportEntries]]: indirectExportEntries,
            indirect_export_entries: Default::default(),
            // [[StarExportEntries]]: starExportEntries,
            star_export_entries: Default::default(),

            source_code,
        }
        // }.
    }
}

/// ### [16.2.1.7 Source Text Module Records](https://tc39.es/ecma262/#sec-source-text-module-records)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SourceTextModule<'a>(u32, PhantomData<&'a GcToken>);

impl core::fmt::Debug for SourceTextModule<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SourceTextModuleRecord {{ index: {}, ... }}", self.0)
    }
}

impl<'m> SourceTextModule<'m> {
    fn get<'a>(self, agent: &'a Agent) -> &'a SourceTextModuleRecord<'m> {
        &agent.heap.source_text_module_records[self.0 as usize]
    }

    fn get_mut<'a>(self, agent: &'a mut Agent) -> &'a mut SourceTextModuleRecord<'static> {
        &mut agent.heap.source_text_module_records[self.0 as usize]
    }

    pub(crate) fn get_index(self) -> usize {
        self.0 as usize
    }

    /// Get the module statements as a slice.
    ///
    /// ## Safety
    ///
    /// The `self` SourceTextModule must be valid (not use-after-free) when
    /// this call is performed, and a copy of it must be rooted (eg. present on
    /// the execution context stack) while the slice is held. A use-after-free
    /// `self` will lead to a panic or the conceptually wrong module's
    /// statements being borrowed from the heap. If the SourceTextModule is not
    /// rooted (at least a copy of it), then the slice will become a dangling
    /// pointer during garbage collection.
    pub(crate) unsafe fn get_statements<'a>(&'a self, agent: &Agent) -> &'a [ast::Statement<'a>] {
        // SAFETY: Caller promises that SourceTextModule is rooted while the
        // statements slice is held: the SourceTextModuleRecord may move during
        // GC but the statements it points to do not move. Hence the reference
        // is valid while the self SourceTextModule is held (the parent call).
        unsafe {
            core::mem::transmute::<&[ast::Statement], &'a [ast::Statement<'a>]>(
                self.get(agent).ecmascript_code.body.as_slice(),
            )
        }
    }

    /// ### \[\[HasTLA]]
    fn has_tla(self, agent: &Agent) -> bool {
        self.get(agent).cyclic_fields.has_tla()
    }

    // ### \[\[Environment]]
    fn environment(self, agent: &Agent) -> ModuleEnvironment<'m> {
        self.get(agent).abstract_fields.environment()
    }

    // Set \[\[Environment]] to env.
    fn set_environment(self, agent: &mut Agent, env: ModuleEnvironment) {
        self.get_mut(agent).abstract_fields.set_environment(env)
    }

    /// ### \[\[EvaluationError]]
    pub(super) fn evaluation_error<'gc>(
        self,
        agent: &Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> JsResult<'gc, ()> {
        self.get(agent).cyclic_fields.evaluation_error(gc)
    }

    /// Set \[\[EvaluationError]] to error and \[\[Status]] to evaluated.
    pub(super) fn set_evaluation_error<'gc>(self, agent: &mut Agent, error: JsError) {
        self.get_mut(agent)
            .cyclic_fields
            .set_evaluation_error(error)
    }

    // ### \[\[Realm]]
    fn realm(self, agent: &Agent) -> Realm<'m> {
        self.get(agent).abstract_fields.realm()
    }

    /// ### \[\[Status]]
    pub(super) fn status<'a>(self, agent: &'a Agent) -> &'a CyclicModuleRecordStatus
    where
        'm: 'a,
    {
        self.get(agent).cyclic_fields.status()
    }

    /// Get a reference to the module's source code storage.
    fn source_code(self, agent: &Agent) -> SourceCode<'m> {
        self.get(agent).source_code
    }

    /// ### \[\[TopLevelCapability]]
    pub(super) fn top_level_capability<'a>(
        self,
        agent: &'a Agent,
    ) -> Option<&'a PromiseCapability<'m>>
    where
        'm: 'a,
    {
        self.get(agent).cyclic_fields.top_level_capability()
    }

    /// Set \[\[DFSIndex]] and \[\[DFSAncestorIndex]] to index.
    pub(super) fn set_dfs_index(self, agent: &mut Agent, index: u32) {
        self.get_mut(agent).cyclic_fields.set_dfs_index(index);
    }

    /// Set module.\[\[Status]] to unlinked.
    pub(super) fn set_unlinked(self, agent: &mut Agent) {
        self.get_mut(agent).cyclic_fields.set_unlinked()
    }

    /// Set module.\[\[Status]] to linking.
    pub(super) fn set_linking(self, agent: &mut Agent) {
        self.get_mut(agent).cyclic_fields.set_linking()
    }

    /// Set module.\[\[Status]] to linked.
    pub(super) fn set_linked(self, agent: &mut Agent) {
        self.get_mut(agent).cyclic_fields.set_linked()
    }

    /// Set module.\[\[Status]] to evaluating.
    pub(super) fn set_evaluating(self, agent: &mut Agent) {
        self.get_mut(agent).cyclic_fields.set_evaluating()
    }
}

impl<'a> From<SourceTextModule<'a>> for ScriptOrModule<'a> {
    fn from(value: SourceTextModule<'a>) -> Self {
        ScriptOrModule::SourceTextModule(value)
    }
}

impl ModuleAbstractMethods for SourceTextModule<'_> {
    /// ### [16.2.1.6.1.1 LoadRequestedModules ( \[ hostDefined \] )](https://tc39.es/ecma262/#sec-LoadRequestedModules)
    ///
    /// The LoadRequestedModules concrete method of a Cyclic Module Record
    /// module takes optional argument hostDefined (anything) and returns a
    /// Promise. It populates the \[\[LoadedModules]] of all the Module Records
    /// in the dependency graph of module (most of the work is done by the
    /// auxiliary function InnerModuleLoading). It takes an optional
    /// hostDefined parameter that is passed to the HostLoadImportedModule
    /// hook.
    ///
    /// > NOTE: The hostDefined parameter can be used to pass additional
    /// > information necessary to fetch the imported modules. It is used, for
    /// > example, by HTML to set the correct fetch destination for
    /// > `<link rel="preload" as="...">` tags. import() expressions never set
    /// > the hostDefined parameter.
    fn load_requested_modules<'a>(
        self,
        agent: &mut Agent,
        host_defined: Option<HostDefined>,
        gc: NoGcScope<'a, '_>,
    ) -> Option<Promise<'a>> {
        // 1. If hostDefined is not present, let hostDefined be empty.
        // 2. Let pc be ! NewPromiseCapability(%Promise%).
        // 3. Let state be the GraphLoadingState Record {
        //        [[IsLoading]]: true,
        //        [[PendingModulesCount]]: 1,
        //        [[Visited]]: « »,
        //        [[PromiseCapability]]: pc,
        //        [[HostDefined]]: hostDefined
        //    }.
        // 4. Perform InnerModuleLoading(state, module).
        // 5. Return pc.[[Promise]].
        self.set_unlinked(agent);
        None
    }

    /// ### [16.2.1.7.2.1 GetExportedNames ( \[ exportStarSet \] )](https://tc39.es/ecma262/#sec-getexportednames)
    ///
    /// The GetExportedNames concrete method of a Source Text Module Record
    /// module takes optional argument exportStarSet (a List of Source Text
    /// Module Records) and returns a List of Strings.
    ///
    /// > NOTE: GetExportedNames does not filter out or throw an exception for
    /// > names that have ambiguous star export bindings.
    fn get_exported_names(self, agent: &mut Agent, export_start_set: Option<()>, gc: GcScope) {
        // 1. Assert: module.[[Status]] is not new.
        // 2. If exportStarSet is not present, set exportStarSet to a new empty List.
        // 3. If exportStarSet contains module, then
        //        a. Assert: We've reached the starting point of an export * circularity.
        //        b. Return a new empty List.
        // 4. Append module to exportStarSet.
        // 5. Let exportedNames be a new empty List.
        // 6. For each ExportEntry Record e of module.[[LocalExportEntries]], do
        //        a. Assert: module provides the direct binding for this export.
        //        b. Assert: e.[[ExportName]] is not null.
        //        c. Append e.[[ExportName]] to exportedNames.
        // 7. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
        //        a. Assert: module imports a specific binding for this export.
        //        b. Assert: e.[[ExportName]] is not null.
        //        c. Append e.[[ExportName]] to exportedNames.
        // 8. For each ExportEntry Record e of module.[[StarExportEntries]], do
        //        a. Assert: e.[[ModuleRequest]] is not null.
        //        b. Let requestedModule be GetImportedModule(module, e.[[ModuleRequest]]).
        //        c. Let starNames be requestedModule.GetExportedNames(exportStarSet).
        //        d. For each element n of starNames, do
        //               i. If n is not "default", then
        //                      1. If exportedNames does not contain n, then
        //                             a. Append n to exportedNames.
        // 9. Return exportedNames.
    }

    /// ### [16.2.1.7.2.2 ResolveExport ( exportName \[ , resolveSet \] )](https://tc39.es/ecma262/#sec-resolveexport)
    fn resolve_export(self, agent: &mut Agent, resolve_set: Option<()>, gc: GcScope) {}

    /// ### [16.2.1.6.1.2 Link ( )](https://tc39.es/ecma262/#sec-moduledeclarationlinking)
    ///
    /// The Link concrete method of a Cyclic Module Record module takes no
    /// arguments and returns either a normal completion containing unused or a
    /// throw completion. On success, Link transitions this module's
    /// \[\[Status]] from unlinked to linked. On failure, an exception is
    /// thrown and this module's \[\[Status]] remains unlinked. (Most of the
    /// work is done by the auxiliary function InnerModuleLinking.)
    fn link<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> JsResult<'a, ()> {
        let module = self.bind(gc);
        // 1. Assert: module.[[Status]] is one of unlinked, linked, evaluating-async, or evaluated.
        debug_assert!(matches!(
            module.status(agent),
            CyclicModuleRecordStatus::Unlinked
                | CyclicModuleRecordStatus::Linked
                | CyclicModuleRecordStatus::EvaluatingAsync
                | CyclicModuleRecordStatus::Evaluated
        ));
        // 2. Let stack be a new empty List.
        let stack = ();
        // 3. Let result be Completion(InnerModuleLinking(module, stack, 0)).
        let result = inner_module_linking(agent, module, stack, 0, gc);
        // 4. If result is an abrupt completion, then
        if let Err(result) = result {
            // a. For each Cyclic Module Record m of stack, do
            //         i. Assert: m.[[Status]] is linking.
            //         ii. Set m.[[Status]] to unlinked.
            module.set_unlinked(agent);
            // b. Assert: module.[[Status]] is unlinked.
            debug_assert!(matches!(
                module.status(agent),
                CyclicModuleRecordStatus::Linking
            ));
            // c. Return ? result.
            return Err(result);
        }
        // 5. Assert: module.[[Status]] is one of linked, evaluating-async, or
        //    evaluated.
        debug_assert!(matches!(
            module.status(agent),
            CyclicModuleRecordStatus::Linked
                | CyclicModuleRecordStatus::EvaluatingAsync
                | CyclicModuleRecordStatus::Evaluated
        ));
        // 6. Assert: stack is empty.
        // 7. Return unused.
        Ok(())
    }

    /// ### [16.2.1.6.1.3 Evaluate ( )](https://tc39.es/ecma262/#sec-moduleevaluation)
    ///
    /// The Evaluate concrete method of a Cyclic Module Record module takes no
    /// arguments and returns a Promise. Evaluate transitions this module's
    /// \[\[Status]] from linked to either evaluating-async or evaluated. The
    /// first time it is called on a module in a given strongly connected
    /// component, Evaluate creates and returns a Promise which resolves when
    /// the module has finished evaluating. This Promise is stored in the
    /// \[\[TopLevelCapability]] field of the \[\[CycleRoot]] for the
    /// component. Future invocations of Evaluate on any module in the
    /// component return the same Promise. (Most of the work is done by the
    /// auxiliary function InnerModuleEvaluation.)
    fn evalute<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> Option<Promise<'gc>> {
        let module = self.bind(gc.nogc());
        // 1. Assert: This call to Evaluate is not happening at the same time
        //    as another call to Evaluate within the surrounding agent.
        // 2. Assert: module.[[Status]] is one of linked, evaluating-async, or
        //    evaluated.
        assert!(matches!(
            module.status(agent),
            CyclicModuleRecordStatus::Linked
                | CyclicModuleRecordStatus::EvaluatingAsync
                | CyclicModuleRecordStatus::Evaluated
        ));
        // 3. If module.[[Status]] is either evaluating-async or evaluated, set
        //    module to module.[[CycleRoot]].
        if matches!(
            module.status(agent),
            CyclicModuleRecordStatus::EvaluatingAsync | CyclicModuleRecordStatus::Evaluated
        ) {
            todo!("set module to module.[[CycleRoot]]");
        }
        // 4. If module.[[TopLevelCapability]] is not empty, then
        if let Some(top_level_capability) = module.top_level_capability(agent) {
            // a. Return module.[[TopLevelCapability]].[[Promise]].
            return Some(top_level_capability.promise.unbind().bind(gc.into_nogc()));
        }
        // 5. Let stack be a new empty List.
        let stack = ();
        // 6. Let capability be ! NewPromiseCapability(%Promise%).
        // 7. Set module.[[TopLevelCapability]] to capability.
        // 8. Let result be Completion(InnerModuleEvaluation(module, stack, 0)).
        let uaf_module = module.unbind();
        let result = inner_module_evaluation(agent, module.unbind(), stack, 0, gc);
        // 9. If result is an abrupt completion, then
        if let Err(result) = result {
            // a. For each Cyclic Module Record m of stack, do
            //        i. Assert: m.[[Status]] is evaluating.
            //        ii. Assert: m.[[AsyncEvaluationOrder]] is unset.
            //        iii. Set m.[[Status]] to evaluated.
            //        iv. Set m.[[EvaluationError]] to result.
            uaf_module.set_evaluation_error(agent, result);
            // b. Assert: module.[[Status]] is evaluated.
            // c. Assert: module.[[EvaluationError]] and result are the same Completion Record.
            // d. Perform ! Call(capability.[[Reject]], undefined, « result.[[Value]] »).
            todo!();
        }
        // 10. Else,
        //         a. Assert: module.[[Status]] is either evaluating-async or evaluated.
        //         b. Assert: module.[[EvaluationError]] is empty.
        //         c. If module.[[Status]] is evaluated, then
        //                i. NOTE: This implies that evaluation of module completed synchronously.
        //                ii. Assert: module.[[AsyncEvaluationOrder]] is unset.
        //                iii. Perform ! Call(capability.[[Resolve]], undefined, « undefined »).
        //         d. Assert: stack is empty.
        // 11. Return capability.[[Promise]].
        None
    }
}

impl CyclicModuleAbstractMethods for SourceTextModule<'_> {
    /// ### [16.2.1.7.3.1 InitializeEnvironment ( )](https://tc39.es/ecma262/#sec-source-text-module-record-initialize-environment)
    ///
    /// The InitializeEnvironment concrete method of a Source Text Module
    /// Record module takes no arguments and returns either a normal
    /// completion containing unused or a throw completion.
    fn initialize_environment<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let module = self.bind(gc);
        // 1. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
        //         a. Assert: e.[[ExportName]] is not null.
        //         b. Let resolution be module.ResolveExport(e.[[ExportName]]).
        //         c. If resolution is either null or ambiguous, throw a SyntaxError exception.
        //         d. Assert: resolution is a ResolvedBinding Record.
        // 2. Assert: All named exports from module are resolvable.
        // 3. Let realm be module.[[Realm]].
        // 4. Assert: realm is not undefined.
        let realm = module.realm(agent);
        // 5. Let env be NewModuleEnvironment(realm.[[GlobalEnv]]).
        let global_env = realm.global_env(agent, gc).unwrap();
        let env = new_module_environment(agent, Some(global_env.into()), gc);
        // 6. Set module.[[Environment]] to env.
        module.set_environment(agent, env);
        // 7. For each ImportEntry Record in of module.[[ImportEntries]], do
        //         a. Let importedModule be GetImportedModule(module, in.[[ModuleRequest]]).
        //         b. If in.[[ImportName]] is namespace-object, then
        //                 i. Let namespace be GetModuleNamespace(importedModule).
        //                 ii. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
        //                 iii. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
        //         c. Else,
        //                 i. Let resolution be importedModule.ResolveExport(in.[[ImportName]]).
        //                 ii. If resolution is either null or ambiguous, throw a SyntaxError exception.
        //                 iii. If resolution.[[BindingName]] is namespace, then
        //                         1. Let namespace be GetModuleNamespace(resolution.[[Module]]).
        //                         2. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
        //                         3. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
        //                 iv. Else,
        //                         1. Perform CreateImportBinding(env, in.[[LocalName]], resolution.[[Module]], resolution.[[BindingName]]).
        // 8. Let moduleContext be a new ECMAScript code execution context.
        let module_context = ExecutionContext {
            ecmascript_code: Some(ECMAScriptCodeEvaluationState {
                // 14. Set the LexicalEnvironment of moduleContext to
                //     module.[[Environment]].
                lexical_environment: env.unbind().into(),
                // 13. Set the VariableEnvironment of moduleContext to
                //     module.[[Environment]].
                variable_environment: env.unbind().into(),
                // 15. Set the PrivateEnvironment of moduleContext to null.
                private_environment: None,
                is_strict_mode: true,
                source_code: module.source_code(agent).unbind(),
            }),
            // 9. Set the Function of moduleContext to null.
            function: None,
            // 10. Assert: module.[[Realm]] is not undefined.
            // 11. Set the Realm of moduleContext to module.[[Realm]].
            realm: realm.unbind(),
            // 12. Set the ScriptOrModule of moduleContext to module.
            script_or_module: Some(ScriptOrModule::SourceTextModule(module.unbind())),
        };
        // 16. Set module.[[Context]] to moduleContext.
        // 17. Push moduleContext onto the execution context stack;
        //     moduleContext is now the running execution context.
        agent.push_execution_context(module_context);
        // 18. Let code be module.[[ECMAScriptCode]].
        // SAFETY: Garbage collection will not be called and module is
        // currently on the execution stack, this is all good.
        let code = unsafe { module.get_statements(agent) };
        // 19. Let varDeclarations be the VarScopedDeclarations of code.
        let var_declarations = module_var_scoped_declarations(code);
        // 20. Let declaredVarNames be a new empty List.
        let mut declared_var_names = AHashSet::with_capacity(var_declarations.len());
        // 21. For each element d of varDeclarations, do
        for d in var_declarations {
            // a. For each element dn of the BoundNames of d, do
            match d {
                VarScopedDeclaration::Variable(d) => {
                    d.id.bound_names(&mut |dn: &oxc_ast::ast::BindingIdentifier| {
                        // i. If declaredVarNames does not contain dn, then
                        let dn = dn.name.as_str();
                        if declared_var_names.insert(dn) {
                            // 3. Append dn to declaredVarNames.
                            let dn = String::from_str(agent, dn, gc);
                            // 1. Perform ! env.CreateMutableBinding(dn, false).
                            env.create_mutable_binding(agent, dn, false);
                            // 2. Perform ! env.InitializeBinding(dn, undefined).
                            env.initialize_binding(agent, dn, Value::Undefined);
                        }
                    })
                }
                VarScopedDeclaration::Function(d) => {
                    d.bound_names(&mut |dn: &oxc_ast::ast::BindingIdentifier| {
                        // i. If declaredVarNames does not contain dn, then
                        let dn = dn.name.as_str();
                        if declared_var_names.insert(dn) {
                            // 3. Append dn to declaredVarNames.
                            let dn = String::from_str(agent, dn, gc);
                            // 1. Perform ! env.CreateMutableBinding(dn, false).
                            env.create_mutable_binding(agent, dn, false);
                            // 2. Perform ! env.InitializeBinding(dn, undefined).
                            env.initialize_binding(agent, dn, Value::Undefined);
                        }
                    })
                }
            }
        }
        // 22. Let lexDeclarations be the LexicallyScopedDeclarations of code.
        let lex_declarations = module_lexically_scoped_declarations(code);
        // 23. Let privateEnv be null.
        let private_env = None;
        // 24. For each element d of lexDeclarations, do
        for d in lex_declarations {
            // a. For each element dn of the BoundNames of d, do
            match d {
                LexicallyScopedDeclaration::Variable(d) => {
                    // i. If IsConstantDeclaration of d is true, then
                    if d.kind.is_const() {
                        d.id.bound_names(&mut |dn| {
                            let dn = dn.name.as_str();
                            let dn = String::from_str(agent, dn, gc);
                            // 1. Perform ! env.CreateImmutableBinding(dn, true).
                            env.create_immutable_binding(agent, dn);
                        });
                    } else {
                        // ii. Else,
                        d.id.bound_names(&mut |dn| {
                            let dn = dn.name.as_str();
                            let dn = String::from_str(agent, dn, gc);
                            // 1. Perform ! env.CreateMutableBinding(dn, false).
                            env.create_mutable_binding(agent, dn, false);
                        });
                    }
                }
                LexicallyScopedDeclaration::Function(d) => {
                    // ii. Else,
                    d.bound_names(&mut |dn| {
                        let dn = dn.name.as_str();
                        let dn = String::from_str(agent, dn, gc);
                        // 1. Perform ! env.CreateMutableBinding(dn, false).
                        env.create_mutable_binding(agent, dn, false);
                        // iii. If d is either a FunctionDeclaration, a
                        //      GeneratorDeclaration, an AsyncFunctionDeclaration,
                        //      or an AsyncGeneratorDeclaration, then
                        // 1. Let fo be InstantiateFunctionObject of d with arguments env and privateEnv.
                        let fo = instantiate_function_object(agent, d, env.into(), private_env, gc);
                        // 2. Perform ! env.InitializeBinding(dn, fo).
                        env.initialize_binding(agent, dn, fo.into_value());
                    });
                }
                LexicallyScopedDeclaration::Class(d) => {
                    // ii. Else,
                    d.bound_names(&mut |dn| {
                        let dn = dn.name.as_str();
                        let dn = String::from_str(agent, dn, gc);
                        // 1. Perform ! env.CreateMutableBinding(dn, false).
                        env.create_mutable_binding(agent, dn, false);
                    });
                }
                LexicallyScopedDeclaration::DefaultExport => {}
            }
        }
        // 25. Remove moduleContext from the execution context stack.
        agent.pop_execution_context();
        // 26. Return unused.
        Ok(())
    }

    /// ### [16.2.1.7.3.2 ExecuteModule ( \[ capability \] )](https://tc39.es/ecma262/#sec-source-text-module-record-execute-module)
    ///
    /// The ExecuteModule concrete method of a Source Text Module Record module
    /// takes optional argument capability (a PromiseCapability Record) and
    /// returns either a normal completion containing unused or a throw
    /// completion.
    fn execute_module<'a>(
        self,
        agent: &mut Agent,
        capability: Option<PromiseCapability>,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let module = self.bind(gc.nogc());
        let capability = capability.bind(gc.nogc());
        // 1. Let moduleContext be a new ECMAScript code execution context.
        // 5. Assert: module has been linked and declarations in its module
        //    environment have been instantiated.
        let environment = module.environment(agent);
        let source_code = module.source_code(agent);
        let module_context = ExecutionContext {
            ecmascript_code: Some(ECMAScriptCodeEvaluationState {
                // 7. Set the LexicalEnvironment of moduleContext to
                //    module.[[Environment]].
                lexical_environment: environment.unbind().into(),
                // 6. Set the VariableEnvironment of moduleContext to
                //    module.[[Environment]].
                variable_environment: environment.unbind().into(),
                private_environment: None,
                is_strict_mode: true,
                source_code: source_code.unbind(),
            }),
            // 2. Set the Function of moduleContext to null.
            function: None,
            // 3. Set the Realm of moduleContext to module.[[Realm]].
            realm: module.realm(agent).unbind(),
            // 4. Set the ScriptOrModule of moduleContext to module.
            script_or_module: Some(module.unbind().into()),
        };

        // 8. Suspend the running execution context.
        // 9. If module.[[HasTLA]] is false, then
        if !module.has_tla(agent) {
            // a. Assert: capability is not present.
            assert!(capability.is_none());
            // b. Push moduleContext onto the execution context stack;
            //    moduleContext is now the running execution context.
            agent.push_execution_context(module_context);
            // c. Let result be Completion(Evaluation of
            //    module.[[ECMAScriptCode]]).
            let bytecode =
                Executable::compile_module(agent, module, gc.nogc()).scope(agent, gc.nogc());
            let result = Vm::execute(agent, bytecode.clone(), None, gc.reborrow())
                .into_js_result()
                .unbind()
                .bind(gc.into_nogc());
            // d. Suspend moduleContext and remove it from the execution
            //    context stack.
            agent.pop_execution_context();
            // e. Resume the context that is now on the top of the execution
            //    context stack as the running execution context.
            // f. If result is an abrupt completion, then
            // i. Return ? result.
            result?;
        } else {
            // 10. Else,
            // a. Assert: capability is a PromiseCapability Record.
            // b. Perform AsyncBlockStart(capability, module.[[ECMAScriptCode]], moduleContext).
            todo!("AsyncBlockStart");
        }
        // 11. Return unused.
        Ok(())
    }
}

pub(crate) type ModuleOrErrors<'a> = Result<SourceTextModule<'a>, Vec<OxcDiagnostic>>;

/// ### [16.2.1.7.1 ParseModule ( sourceText, realm, hostDefined )](https://tc39.es/ecma262/#sec-parsemodule)
pub(crate) fn parse_module<'a>(
    agent: &mut Agent,
    source_text: String,
    realm: Realm,
    host_defined: Option<HostDefined>,
    gc: NoGcScope<'a, '_>,
) -> ModuleOrErrors<'a> {
    let realm = realm.bind(gc);
    // 1. Let body be ParseText(sourceText, Module).
    let source_type = if cfg!(feature = "typescript") {
        SourceType::default()
            .with_module(true)
            .with_typescript(true)
    } else {
        SourceType::default().with_module(true)
    };
    // SAFETY: Script keeps the SourceCode reference alive in the Heap, thus
    // making the Program's references point to a live Allocator.
    let parse_result = unsafe { SourceCode::parse_source(agent, source_text, source_type, gc) };

    let (body, source_code) = match parse_result {
        // 2. If body is a List of errors, return body.
        Ok(result) => result,
        Err(errors) => {
            return Err(errors);
        }
    };

    // 3. Let requestedModules be the ModuleRequests of body.
    let requested_modules = ();
    // 4. Let importEntries be the ImportEntries of body.
    // 5. Let importedBoundNames be ImportedLocalNames(importEntries).
    // 6. Let indirectExportEntries be a new empty List.
    // 7. Let localExportEntries be a new empty List.
    // 8. Let starExportEntries be a new empty List.
    // 9. Let exportEntries be the ExportEntries of body.
    // 10. For each ExportEntry Record ee of exportEntries, do
    //         a. If ee.[[ModuleRequest]] is null, then
    //                i. If importedBoundNames does not contain ee.[[LocalName]], then
    //                       1. Append ee to localExportEntries.
    //                ii. Else,
    //                        1. Let ie be the element of importEntries whose [[LocalName]] is ee.[[LocalName]].
    //                        2. If ie.[[ImportName]] is namespace-object, then
    //                               a. NOTE: This is a re-export of an imported module namespace object.
    //                               b. Append ee to localExportEntries.
    //                        3. Else,
    //                               a. NOTE: This is a re-export of a single name.
    //                               b. Append the ExportEntry Record { [[ModuleRequest]]: ie.[[ModuleRequest]], [[ImportName]]: ie.[[ImportName]], [[LocalName]]: null, [[ExportName]]: ee.[[ExportName]] } to indirectExportEntries.
    //         b. Else if ee.[[ImportName]] is all-but-default, then
    //                i. Assert: ee.[[ExportName]] is null.
    //                ii. Append ee to starExportEntries.
    //         c. Else,
    //                i. Append ee to indirectExportEntries.

    // 11. Let async be body Contains await.
    let r#async = false;
    // 12. Return Source Text Module Record {
    Ok(agent.heap.create(SourceTextModuleRecord::new(
        realm,
        host_defined,
        r#async,
        requested_modules,
        body,
        source_code,
    )))
    // }.
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SourceTextModuleRecord<'_> {
    type Of<'a> = SourceTextModuleRecord<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<_, _>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<_, _>(self) }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SourceTextModule<'_> {
    type Of<'a> = SourceTextModule<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<_, _>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<_, _>(self) }
    }
}

impl<'a> CreateHeapData<SourceTextModuleRecord<'a>, SourceTextModule<'a>> for Heap {
    fn create(&mut self, data: SourceTextModuleRecord<'a>) -> SourceTextModule<'a> {
        let index = u32::try_from(self.source_text_module_records.len())
            .expect("SourceTextModuleRecord count overflowed");
        self.source_text_module_records.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<SourceTextModuleRecord<'static>>();
        SourceTextModule(index, PhantomData)
    }
}

impl HeapMarkAndSweep for SourceTextModule<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.source_text_module_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .source_text_module_records
            .shift_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for SourceTextModuleRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            abstract_fields,
            cyclic_fields,
            ecmascript_code: _,
            context: _,
            import_meta,
            import_entries: _,
            local_export_entries: _,
            indirect_export_entries: _,
            star_export_entries: _,
            source_code,
        } = self;
        abstract_fields.mark_values(queues);
        cyclic_fields.mark_values(queues);
        import_meta.mark_values(queues);
        source_code.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            abstract_fields,
            cyclic_fields,
            ecmascript_code: _,
            context: _,
            import_meta,
            import_entries: _,
            local_export_entries: _,
            indirect_export_entries: _,
            star_export_entries: _,
            source_code,
        } = self;
        abstract_fields.sweep_values(compactions);
        cyclic_fields.sweep_values(compactions);
        import_meta.sweep_values(compactions);
        source_code.sweep_values(compactions);
    }
}
