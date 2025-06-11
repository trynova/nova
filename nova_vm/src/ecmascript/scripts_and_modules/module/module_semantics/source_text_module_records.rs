// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2.1.7 Source Text Module Records](https://tc39.es/ecma262/#sec-source-text-module-records)

use std::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

use ahash::AHashSet;
use oxc_ast::ast::{self, Program};
use oxc_diagnostics::OxcDiagnostic;
use oxc_ecmascript::BoundNames;
use oxc_span::SourceType;
use small_string::SmallString;

use crate::{
    ecmascript::{
        builtins::{
            module::Module, promise::Promise,
            promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability,
        },
        execution::{
            Agent, ECMAScriptCodeEvaluationState, ExecutionContext, JsResult, ModuleEnvironment,
            Realm,
            agent::{ExceptionType, JsError},
            create_import_binding, initialize_import_binding, new_module_environment,
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
        types::{
            BUILTIN_STRING_MEMORY, HeapString, IntoValue, Object, SMALL_STRING_DISCRIMINANT,
            STRING_DISCRIMINANT, String, Value,
        },
    },
    engine::{
        Executable, Scoped, Vm,
        context::{Bindable, GcScope, GcToken, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable, Scopable},
    },
    heap::{CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues},
};

use super::{
    ModuleRequestRecord,
    abstract_module_records::{AbstractModuleRecord, ModuleAbstractMethods, ResolvedBinding},
    cyclic_module_records::{
        CyclicModuleAbstractMethods, CyclicModuleRecord, GraphLoadingStateRecord,
        inner_module_loading,
    },
    get_imported_module, get_module_namespace,
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
    /// An object exposed through the import.meta meta property. It is empty
    /// until it is accessed by ECMAScript code.
    import_meta: Option<Object<'a>>,
    /// ### \[\[ImportEntries]]
    ///
    /// A List of ImportEntry records derived from the code of this module.
    import_entries: Box<[ImportEntryRecord<'a>]>,
    /// ### \[\[LocalExportEntries]]
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to declarations that occur within the module.
    local_export_entries: Box<[LocalExportEntryRecord<'a>]>,
    /// ### \[\[IndirectExportEntries]]
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to reexported imports that occur within the module or
    /// exports from export * as namespace declarations.
    indirect_export_entries: Box<[IndirectExportEntryRecord<'a>]>,
    /// ### \[\[StarExportEntries]]
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to export * declarations that occur within the module, not
    /// including export * as namespace declarations.
    star_export_entries: Box<[ModuleRequestRecord<'a>]>,

    /// Source text of the script
    ///
    /// The source text is kept in the heap strings vector, through the
    /// SourceCode struct.
    pub(crate) source_code: SourceCode<'a>,
}

/// ### [16.2.1.7 Source Text Module Records](https://tc39.es/ecma262/#sec-source-text-module-records)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceTextModule<'a>(u32, PhantomData<&'a GcToken>);

impl core::fmt::Debug for SourceTextModule<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SourceTextModuleRecord {{ index: {}, ... }}", self.0)
    }
}

impl<'m> SourceTextModule<'m> {
    pub(crate) const fn _def() -> Self {
        Self(0, PhantomData)
    }

    fn get<'a>(
        self,
        agent: &'a impl AsRef<SourceTextModuleHeap>,
    ) -> &'a SourceTextModuleRecord<'m> {
        &agent.as_ref()[self.0 as usize]
    }

    fn get_mut(self, agent: &mut Agent) -> &mut SourceTextModuleRecord<'static> {
        &mut agent.heap.source_text_module_records[self.0 as usize]
    }

    pub(crate) fn get_index(self) -> usize {
        self.0 as usize
    }

    /// Get a loaded module by a request string.
    pub(super) fn get_loaded_module(
        self,
        agent: &impl AsRef<SourceTextModuleHeap>,
        request: &ModuleRequestRecord,
    ) -> Option<SourceTextModule<'m>> {
        self.get(agent).cyclic_fields.get_loaded_module(request)
    }

    /// Insert a loaded module into the module's requested modules.
    pub(super) fn insert_loaded_module(
        self,
        agent: &mut Agent,
        request: &ModuleRequestRecord,
        module: SourceTextModule,
    ) {
        self.get_mut(agent)
            .cyclic_fields
            .insert_loaded_module(request, module)
    }

    /// Get the requested modules as a slice.
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
    pub(super) unsafe fn get_requested_modules(
        self,
        agent: &Agent,
    ) -> &'m [ModuleRequestRecord<'m>] {
        // SAFETY: Caller promises that SourceTextModule is rooted while the
        // statements slice is held: the SourceTextModuleRecord may move during
        // GC but the Atoms list memory does not move. Hence the reference is
        // valid while the self SourceTextModule is held (the parent call).
        unsafe {
            core::mem::transmute::<&[ModuleRequestRecord], &'m [ModuleRequestRecord<'m>]>(
                self.get(agent).cyclic_fields.get_requested_modules(),
            )
        }
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

    /// ### \[\[ImportEntries]]
    fn import_entries(
        self,
        agent: &impl AsRef<SourceTextModuleHeap>,
    ) -> &'m [ImportEntryRecord<'m>] {
        // SAFETY: import_entries list cannot be mutated except for during GC.
        // Hence, we can safely transmute to the module lifetime (which should
        // be bound to the GcScope).
        unsafe {
            core::mem::transmute::<&[ImportEntryRecord], &'m [ImportEntryRecord<'m>]>(
                &self.get(agent).import_entries,
            )
        }
    }

    /// ### \[\[LocalExportEntries]]
    fn local_export_entries(
        self,
        agent: &impl AsRef<SourceTextModuleHeap>,
    ) -> &[LocalExportEntryRecord<'m>] {
        &self.get(agent).local_export_entries
    }

    /// ### \[\[IndirectExportEntries]]
    fn indirect_export_entries(
        self,
        agent: &impl AsRef<SourceTextModuleHeap>,
    ) -> &[IndirectExportEntryRecord<'m>] {
        &self.get(agent).indirect_export_entries
    }

    /// ### \[\[StarExportEntries]]
    fn star_export_entries(
        self,
        agent: &impl AsRef<SourceTextModuleHeap>,
    ) -> &[ModuleRequestRecord<'m>] {
        &self.get(agent).star_export_entries
    }

    /// ### \[\[HasTLA]]
    fn has_tla(self, agent: &Agent) -> bool {
        self.get(agent).cyclic_fields.has_tla()
    }

    // ### \[\[Environment]]
    pub(crate) fn environment(
        self,
        agent: &impl AsRef<SourceTextModuleHeap>,
    ) -> Option<ModuleEnvironment<'m>> {
        self.get(agent).abstract_fields.environment()
    }

    // Set \[\[Environment]] to env.
    fn set_environment(self, agent: &mut Agent, env: ModuleEnvironment) {
        self.get_mut(agent).abstract_fields.set_environment(env)
    }

    /// ### \[\[Namespace]]
    pub(crate) fn namespace(self, agent: &Agent) -> Option<Module<'m>> {
        self.get(agent).abstract_fields.namespace()
    }

    // Set \[\[Namespace]] to namespace.
    pub(crate) fn set_namespace(self, agent: &mut Agent, namespace: Module) {
        self.get_mut(agent).abstract_fields.set_namespace(namespace)
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
    pub(super) fn set_evaluation_error(self, agent: &mut Agent, error: JsError) {
        self.get_mut(agent)
            .cyclic_fields
            .set_evaluation_error(error)
    }

    // ### \[\[Realm]]
    pub fn realm(self, agent: &Agent) -> Realm<'m> {
        self.get(agent).abstract_fields.realm()
    }

    /// ### \[\[Status]]
    pub(super) fn status<'a>(
        self,
        agent: &'a impl AsRef<SourceTextModuleHeap>,
    ) -> &'a CyclicModuleRecordStatus
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

    /// ### \[\[DFSAncestorIndex]]
    pub(super) fn dfs_ancestor_index(self, agent: &Agent) -> u32 {
        self.get(agent).cyclic_fields.dfs_ancestor_index()
    }

    /// Set \[\[DFSAncestorIndex]] to value if it is larger than previous.
    pub(super) fn set_dfs_ancestor_index(self, agent: &mut Agent, value: u32) {
        self.get_mut(agent)
            .cyclic_fields
            .set_dfs_ancestor_index(value);
    }

    /// ### \[\[DFSIndex]]
    pub(super) fn dfs_index(self, agent: &Agent) -> u32 {
        self.get(agent).cyclic_fields.dfs_index()
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

    /// Set module.\[\[Status]] to evaluated.
    pub(super) fn set_evaluated(self, agent: &mut Agent) {
        self.get_mut(agent).cyclic_fields.set_evaluated()
    }

    fn inner_resolve_export<'a>(
        self,
        agent: &impl AsRef<SourceTextModuleHeap>,
        export_name: String,
        resolve_set: Option<()>,
        gc: NoGcScope<'a, '_>,
    ) -> Option<ResolvedBinding<'a>> {
        let module = self.bind(gc);
        // 1. Assert: module.[[Status]] is not new.
        debug_assert!(!matches!(
            module.status(agent),
            CyclicModuleRecordStatus::New
        ));
        // 2. If resolveSet is not present, set resolveSet to a new empty List.
        // 3. For each Record { [[Module]], [[ExportName]] } r of resolveSet, do
        //        a. If module and r.[[Module]] are the same Module Record and exportName is r.[[ExportName]], then
        //               i. Assert: This is a circular import request.
        //               ii. Return null.

        // 4. Append the Record { [[Module]]: module, [[ExportName]]: exportName } to resolveSet.
        // 5. For each ExportEntry Record e of module.[[LocalExportEntries]], do
        for e in module.local_export_entries(agent) {
            // a. If e.[[ExportName]] is exportName, then
            if e.export_name == export_name {
                // i. Assert: module provides the direct binding for this export.
                // ii. Return ResolvedBinding Record {
                return Some(ResolvedBinding::Resolved {
                    // [[Module]]: module,
                    module,
                    // [[BindingName]]: e.[[LocalName]]
                    binding_name: Some(e.local_name),
                });
                // }.
            }
        }

        // 6. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
        for e in module.indirect_export_entries(agent) {
            // a. If e.[[ExportName]] is exportName, then
            if e.export_name == export_name {
                // i. Assert: e.[[ModuleRequest]] is not null.
                // ii. Let importedModule be GetImportedModule(module, e.[[ModuleRequest]]).
                let imported_module = get_imported_module(agent, module, &e.module_request, gc);
                // iii. If e.[[ImportName]] is all, then
                if let Some(import_name) = e.import_name {
                    // iv. Else,
                    // 1. Assert: module imports a specific binding for this export.
                    // 2. Assert: e.[[ImportName]] is a String.
                    // 3. Return importedModule.ResolveExport(e.[[ImportName]], resolveSet).
                    return imported_module.inner_resolve_export(
                        agent,
                        import_name,
                        resolve_set,
                        gc,
                    );
                } else {
                    // 1. Assert: module does not provide the direct binding for this export.
                    // 2. Return ResolvedBinding Record { [[Module]]: importedModule, [[BindingName]]: namespace }.
                    return Some(ResolvedBinding::Resolved {
                        module: imported_module,
                        binding_name: None,
                    });
                }
            }
        }

        // 7. If exportName is "default", then
        if export_name == BUILTIN_STRING_MEMORY.default {
            // a. Assert: A default export was not explicitly defined by this
            //    module.
            // b. Return null.
            // c. NOTE: A default export cannot be provided by an export * from
            //    "mod" declaration.
            return None;
        }

        // 8. Let starResolution be null.
        let mut star_resolution = None;
        // 9. For each ExportEntry Record e of module.[[StarExportEntries]], do
        for e in module.star_export_entries(agent) {
            // a. Assert: e.[[ModuleRequest]] is not null.
            // b. Let importedModule be GetImportedModule(module, e.[[ModuleRequest]]).
            let imported_module = get_imported_module(agent, module, e, gc);
            // c. Let resolution be importedModule.ResolveExport(exportName, resolveSet).
            let resolution =
                imported_module.inner_resolve_export(agent, export_name, resolve_set, gc);
            // d. If resolution is ambiguous, return ambiguous.
            if matches!(resolution, Some(ResolvedBinding::Ambiguous)) {
                return resolution;
            }
            // e. If resolution is not null, then
            if let Some(resolution) = resolution {
                // i. Assert: resolution is a ResolvedBinding Record.
                let ResolvedBinding::Resolved {
                    module: resolution_module,
                    binding_name: resolution_binding_name,
                } = resolution
                else {
                    unreachable!()
                };
                debug_assert!(matches!(resolution, ResolvedBinding::Resolved { .. }));
                // ii. If starResolution is null, then
                if star_resolution.is_none() {
                    // 1. Set starResolution to resolution.
                    star_resolution = Some(resolution);
                } else {
                    let Some(ResolvedBinding::Resolved {
                        module: star_resolution_module,
                        binding_name: star_resolution_binding_name,
                    }) = star_resolution
                    else {
                        unreachable!()
                    };
                    // iii. Else,
                    // 1. Assert: There is more than one * import that includes
                    // the requested name.
                    debug_assert!(module.star_export_entries(agent).len() > 1);
                    // 2. If resolution.[[Module]] and starResolution.[[Module]]
                    //    are not the same Module Record, return ambiguous.
                    if resolution_module != star_resolution_module {
                        return Some(ResolvedBinding::Ambiguous);
                    }
                    // 3. If resolution.[[BindingName]] is not
                    //    starResolution.[[BindingName]] and either
                    //    resolution.[[BindingName]] or
                    //    starResolution.[[BindingName]] is namespace, return
                    //    ambiguous.
                    if resolution_binding_name != star_resolution_binding_name
                        && (resolution_binding_name.is_none()
                            || star_resolution_binding_name.is_none())
                    {
                        return Some(ResolvedBinding::Ambiguous);
                    }
                    // 4. If resolution.[[BindingName]] is a String,
                    //    starResolution.[[BindingName]] is a String, and
                    //    resolution.[[BindingName]] is not
                    //    starResolution.[[BindingName]], return ambiguous.
                    if resolution_binding_name.is_some()
                        && star_resolution_binding_name.is_some()
                        && resolution_binding_name != star_resolution_binding_name
                    {
                        return Some(ResolvedBinding::Ambiguous);
                    }
                }
            }
        }

        // 10. Return starResolution.
        None
    }
}

impl Scoped<'_, SourceTextModule<'static>> {
    /// Get the requested modules as a slice.
    pub(super) fn get_requested_modules<'a>(
        &'a self,
        agent: &Agent,
    ) -> &'a [ModuleRequestRecord<'a>] {
        // SAFETY: Scoped roots the SourceTextModule.
        unsafe { self.get(agent).get_requested_modules(agent) }
    }
}

/// ### [ImportEntry Record Fields](https://tc39.es/ecma262/#table-importentry-record-fields)
#[derive(Debug)]
struct ImportEntryRecord<'a> {
    /// ### \[\[ModuleRequest]]
    ///
    /// a ModuleRequest Record
    ///
    /// ModuleRequest Record representing the ModuleSpecifier and import
    /// attributes of the ImportDeclaration.
    module_request: ModuleRequestRecord<'a>,
    /// ### \[\[ImportName]]
    ///
    /// The name under which the desired binding is exported by the module
    /// identified by \[\[ModuleRequest]]. The value namespace-object indicates
    /// that the import request is for the target module's namespace object.
    ///
    /// Note: If the \[\[ImportName]] is NAMESPACE-OBJECT, then the value is
    /// None.
    import_name: Option<String<'a>>,
    /// ### \[\[LocalName]]
    ///
    /// The name that is used to locally access the imported value from within
    /// the importing module.
    local_name: String<'a>,
}

/// ### \[\[ImportName]]
///
/// The name under which the desired binding is exported by the module
/// identified by \[\[ModuleRequest]]. null if the ExportDeclaration does
/// not have a ModuleSpecifier. all is used for `export * as ns from "mod"`
/// declarations. all-but-default is used for `export * from "mod"`
/// declarations.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum ExportEntryImportName<'a> {
    /// ```javascript
    /// export * as ns from "mod";
    /// ```
    All = 1,
    /// ```javascript
    /// export * from "mod";
    /// ```
    AllButDefault = 2,
    String(HeapString<'a>) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
}

impl<'a> From<String<'a>> for ExportEntryImportName<'a> {
    fn from(value: String<'a>) -> Self {
        match value {
            String::String(s) => Self::String(s),
            String::SmallString(s) => Self::SmallString(s),
        }
    }
}

/// ## [ExportEntry Record Fields](https://tc39.es/ecma262/#table-exportentry-records)
///
/// This struct is used for local export declarations.
///
/// ### Examples
///
/// ```javascript
/// export x;
/// export var x;
/// export let x;
/// export const x;
/// export function x() {}
/// export class X {}
/// ```
#[derive(Debug, Clone, Copy)]
struct LocalExportEntryRecord<'a> {
    /// ### \[\[ExportName]]
    ///
    /// The name used to export this binding by this module.
    export_name: String<'a>,
    /// ### \[\[LocalName]]
    ///
    /// The name that is used to locally access the exported value from within
    /// the importing module.
    local_name: String<'a>,
}

/// ## [ExportEntry Record Fields](https://tc39.es/ecma262/#table-exportentry-records)
///
/// This struct is used for re-export declarations.
///
/// ### Examples
///
/// ```javascript
/// export * as ns from "mod";
/// export { x } from "mod";
/// export { v as x } from "mod";
/// ```
#[derive(Debug, Clone, Copy)]
struct IndirectExportEntryRecord<'a> {
    /// ### \[\[ExportName]]
    ///
    /// The name used to export this binding by this module.
    export_name: String<'a>,
    /// ### \[\[ModuleRequest]]
    ///
    /// The ModuleRequest Record representing the ModuleSpecifier and import
    /// attributes of the ExportDeclaration.
    module_request: ModuleRequestRecord<'a>,
    /// ### \[\[ImportName]]
    ///
    /// The name under which the desired binding is exported by the module
    /// identified by \[\[ModuleRequest]]. None is used for
    /// `export * as ns from "mod"` declarations.
    import_name: Option<String<'a>>,
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for IndirectExportEntryRecord<'_> {
    type Of<'a> = IndirectExportEntryRecord<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<_, _>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<_, _>(self) }
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
    ) -> Promise<'a> {
        let module = self.bind(gc);
        // 1. If hostDefined is not present, let hostDefined be empty.
        // 2. Let pc be ! NewPromiseCapability(%Promise%).
        // 3. Let state be the GraphLoadingState Record {
        let mut state = GraphLoadingStateRecord {
            // [[PromiseCapability]]: pc,
            promise_capability: PromiseCapability::new(agent, gc),
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
        inner_module_loading(agent, &mut state, module, gc);
        // 5. Return pc.[[Promise]].
        state.promise_capability.promise()
    }

    /// ### [16.2.1.7.2.1 GetExportedNames ( \[ exportStarSet \] )](https://tc39.es/ecma262/#sec-getexportednames)
    ///
    /// The GetExportedNames concrete method of a Source Text Module Record
    /// module takes optional argument exportStarSet (a List of Source Text
    /// Module Records) and returns a List of Strings.
    ///
    /// > NOTE: GetExportedNames does not filter out or throw an exception for
    /// > names that have ambiguous star export bindings.
    fn get_exported_names<'a>(
        self,
        agent: &Agent,
        export_start_set: &mut Vec<SourceTextModule<'a>>,
        gc: NoGcScope<'a, '_>,
    ) -> Vec<String<'a>> {
        let module = self.bind(gc);
        // 1. Assert: module.[[Status]] is not new.
        debug_assert!(!matches!(
            module.status(agent),
            CyclicModuleRecordStatus::New
        ));
        // 2. If exportStarSet is not present, set exportStarSet to a new empty
        //    List.
        // 3. If exportStarSet contains module, then
        if export_start_set.contains(&module) {
            // a. Assert: We've reached the starting point of an export * circularity.
            // b. Return a new empty List.
            return vec![];
        }
        // 4. Append module to exportStarSet.
        export_start_set.push(module);
        // 5. Let exportedNames be a new empty List.
        let mut exported_names = vec![];
        // 6. For each ExportEntry Record e of module.[[LocalExportEntries]], do
        for e in module.local_export_entries(agent) {
            // a. Assert: module provides the direct binding for this export.
            // b. Assert: e.[[ExportName]] is not null.
            // c. Append e.[[ExportName]] to exportedNames.
            exported_names.push(e.export_name);
        }
        // 7. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
        for e in module.indirect_export_entries(agent) {
            // a. Assert: module imports a specific binding for this export.
            // b. Assert: e.[[ExportName]] is not null.
            // c. Append e.[[ExportName]] to exportedNames.
            exported_names.push(e.export_name);
        }
        // 8. For each ExportEntry Record e of module.[[StarExportEntries]], do
        for e in module.star_export_entries(agent) {
            // a. Assert: e.[[ModuleRequest]] is not null.
            // b. Let requestedModule be GetImportedModule(module, e.[[ModuleRequest]]).
            let requested_module = get_imported_module(agent, module, &e, gc);
            // c. Let starNames be requestedModule.GetExportedNames(exportStarSet).
            let start_names = requested_module.get_exported_names(agent, export_start_set, gc);
            // d. For each element n of starNames, do
            for n in start_names {
                // i. If n is not "default", then
                if n != BUILTIN_STRING_MEMORY.default {
                    // 1. If exportedNames does not contain n, then
                    if !exported_names.contains(&n) {
                        // a. Append n to exportedNames.
                        exported_names.push(n);
                    }
                }
            }
        }
        // 9. Return exportedNames.
        exported_names
    }

    /// ### [16.2.1.7.2.2 ResolveExport ( exportName \[ , resolveSet \] )](https://tc39.es/ecma262/#sec-resolveexport)
    ///
    /// The ResolveExport concrete method of a Source Text Module Record module
    /// takes argument exportName (a String) and optional argument resolveSet
    /// (a List of Records with fields \[\[Module]] (a Module Record) and
    /// \[\[ExportName]] (a String)) and returns a ResolvedBinding Record,
    /// null, or ambiguous.
    ///
    /// ResolveExport attempts to resolve an imported binding to the actual
    /// defining module and local binding name. The defining module may be the
    /// module represented by the Module Record this method was invoked on or
    /// some other module that is imported by that module. The parameter
    /// resolveSet is used to detect unresolved circular import/export paths.
    /// If a pair consisting of specific Module Record and exportName is
    /// reached that is already in resolveSet, an import circularity has been
    /// encountered. Before recursively calling ResolveExport, a pair
    /// consisting of module and exportName is added to resolveSet.
    ///
    /// If a defining module is found, a ResolvedBinding Record { \[\[Module]],
    /// \[\[BindingName]] } is returned. This record identifies the resolved
    /// binding of the originally requested export, unless this is the export
    /// of a namespace with no local binding. In this case, \[\[BindingName]]
    /// will be set to namespace. If no definition was found or the request is
    /// found to be circular, null is returned. If the request is found to be
    /// ambiguous, ambiguous is returned.
    fn resolve_export<'a>(
        self,
        agent: &Agent,
        export_name: String,
        resolve_set: Option<()>,
        gc: NoGcScope<'a, '_>,
    ) -> Option<ResolvedBinding<'a>> {
        self.inner_resolve_export(agent, export_name, resolve_set, gc)
    }

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
        let mut stack = Vec::with_capacity(8);
        // 3. Let result be Completion(InnerModuleLinking(module, stack, 0)).
        let result = inner_module_linking(agent, module, &mut stack, 0, gc);
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
    fn evaluate<'gc>(self, agent: &mut Agent, mut gc: GcScope<'gc, '_>) -> Option<Promise<'gc>> {
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
        let mut stack = vec![];
        // 6. Let capability be ! NewPromiseCapability(%Promise%).
        // 7. Set module.[[TopLevelCapability]] to capability.
        // 8. Let result be Completion(InnerModuleEvaluation(module, stack, 0)).
        let scoped_module = module.scope(agent, gc.nogc());
        let result =
            inner_module_evaluation(agent, scoped_module.clone(), &mut stack, 0, gc.reborrow())
                .unbind()
                .bind(gc.nogc());
        let module = scoped_module.get(agent).bind(gc.nogc());
        // 9. If result is an abrupt completion, then
        if let Err(result) = result {
            // a. For each Cyclic Module Record m of stack, do
            for m in stack {
                let m = m.get(agent).bind(gc.nogc());
                // i. Assert: m.[[Status]] is evaluating.
                debug_assert!(matches!(
                    m.status(agent),
                    CyclicModuleRecordStatus::Evaluating
                ));
                // ii. Assert: m.[[AsyncEvaluationOrder]] is unset.
                // iii. Set m.[[Status]] to evaluated.
                // iv. Set m.[[EvaluationError]] to result.
                m.set_evaluation_error(agent, result);
            }
            // b. Assert: module.[[Status]] is evaluated.
            debug_assert!(matches!(
                module.status(agent),
                CyclicModuleRecordStatus::Evaluated
            ));
            // c. Assert: module.[[EvaluationError]] and result are the same Completion Record.
            debug_assert_eq!(module.evaluation_error(agent, gc.nogc()), Err(result));
            // d. Perform ! Call(capability.[[Reject]], undefined, « result.[[Value]] »).
            return Some(Promise::new_rejected(
                agent,
                result.value().unbind(),
                gc.into_nogc(),
            ));
        }
        // 10. Else,
        // a. Assert: module.[[Status]] is either evaluating-async or evaluated.
        debug_assert!(matches!(
            module.status(agent),
            CyclicModuleRecordStatus::EvaluatingAsync | CyclicModuleRecordStatus::Evaluated
        ));
        // b. Assert: module.[[EvaluationError]] is empty.
        debug_assert!(module.evaluation_error(agent, gc.nogc()).is_ok());
        // d. Assert: stack is empty.
        debug_assert!(stack.is_empty());
        // c. If module.[[Status]] is evaluated, then
        if matches!(module.status(agent), CyclicModuleRecordStatus::Evaluated) {
            // i. NOTE: This implies that evaluation of module completed
            //    synchronously.
            // ii. Assert: module.[[AsyncEvaluationOrder]] is unset.
            // iii. Perform ! Call(capability.[[Resolve]], undefined, « undefined »).
            None
        } else {
            // 11. Return capability.[[Promise]].
            todo!()
        }
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
        for e in module.indirect_export_entries(agent) {
            // a. Assert: e.[[ExportName]] is not null.
            // b. Let resolution be module.ResolveExport(e.[[ExportName]]).
            let resolution = module.resolve_export(agent, e.export_name, None, gc);
            // c. If resolution is either null or ambiguous, throw a SyntaxError exception.
            if matches!(resolution, None | Some(ResolvedBinding::Ambiguous)) {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::SyntaxError,
                    "ambiguous imports",
                    gc,
                ));
            }
            // d. Assert: resolution is a ResolvedBinding Record.
        }
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
        for r#in in module.import_entries(agent) {
            // a. Let importedModule be GetImportedModule(module, in.[[ModuleRequest]]).
            let imported_modules = get_imported_module(agent, module, &r#in.module_request, gc);
            // b. If in.[[ImportName]] is namespace-object, then
            let Some(import_name) = r#in.import_name else {
                // i. Let namespace be GetModuleNamespace(importedModule).
                // ii. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
                // iii. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
                todo!();
            };
            // c. Else,
            // i. Let resolution be importedModule.ResolveExport(in.[[ImportName]]).
            let resolution = imported_modules.inner_resolve_export(agent, import_name, None, gc);
            // ii. If resolution is either null or ambiguous, throw a SyntaxError exception.
            let Some(ResolvedBinding::Resolved {
                module,
                binding_name,
            }) = resolution
            else {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::SyntaxError,
                    "resolution is null or ambiguous",
                    gc,
                ));
            };
            // iii. If resolution.[[BindingName]] is namespace, then
            let Some(binding_name) = binding_name else {
                // 1. Let namespace be GetModuleNamespace(resolution.[[Module]]).
                let namespace = get_module_namespace(agent, module, gc);
                // 2. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
                env.create_immutable_binding(agent, r#in.local_name);
                // 3. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
                env.initialize_binding(agent, r#in.local_name, namespace.into_value());
                continue;
            };
            // iv. Else,
            // 1. Perform CreateImportBinding(env, in.[[LocalName]], resolution.[[Module]], resolution.[[BindingName]]).
            create_import_binding(agent, env, r#in.local_name, module, binding_name);
        }
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
                            env.initialize_binding(
                                &mut agent.heap.environments,
                                dn,
                                Value::Undefined,
                            );
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
                            env.initialize_binding(
                                &mut agent.heap.environments,
                                dn,
                                Value::Undefined,
                            );
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
                            env.create_immutable_binding(&mut agent.heap.environments, dn);
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
                        env.initialize_binding(&mut agent.heap.environments, dn, fo.into_value());
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
                LexicallyScopedDeclaration::DefaultExport => {
                    // ExportDeclaration : export default AssignmentExpression ;
                    // 1. Return « "*default*" ».
                    // NOTE: It is not necessary to treat export default
                    // AssignmentExpression as a constant declaration because
                    // there is no syntax that permits assignment to the
                    // internal bound name used to reference a module's default
                    // object.
                    // NOTE: We optimise references to constant declarations
                    // separately, so we choose to use an immutable binding
                    // here despite the spec suggesting a mutable one.
                    env.create_immutable_binding(agent, BUILTIN_STRING_MEMORY._default_);
                }
            }
        }
        // 25. Remove moduleContext from the execution context stack.
        agent.pop_execution_context();
        // 26. Return unused.
        Ok(())
    }

    fn bind_environment(self, agent: &mut Agent, gc: NoGcScope) {
        let module = self.bind(gc);
        let env = module
            .environment(agent)
            .expect("Attempted to bind environment of unlinked module");
        let envs = &mut agent.heap.environments;
        let source_text_modules = &agent.heap.source_text_module_records;
        for r#in in module.import_entries(source_text_modules) {
            // NOTE: Spec-text from InitializeEnvironment to contrast with what
            // we're doing here.
            // a. Let importedModule be GetImportedModule(module, in.[[ModuleRequest]]).
            let imported_modules =
                get_imported_module(source_text_modules, module, &r#in.module_request, gc);
            // b. If in.[[ImportName]] is namespace-object, then
            let Some(import_name) = r#in.import_name else {
                // i. Let namespace be GetModuleNamespace(importedModule).
                // ii. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
                // iii. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
                continue;
            };
            // c. Else,
            // i. Let resolution be importedModule.ResolveExport(in.[[ImportName]]).
            let resolution =
                imported_modules.inner_resolve_export(source_text_modules, import_name, None, gc);
            // ii. If resolution is either null or ambiguous, throw a SyntaxError exception.
            let Some(ResolvedBinding::Resolved {
                module,
                binding_name,
            }) = resolution
            else {
                // Note: we've already thrown the SyntaxError earlier.
                unreachable!();
            };
            // iii. If resolution.[[BindingName]] is namespace, then
            let Some(binding_name) = binding_name else {
                // 1. Let namespace be GetModuleNamespace(resolution.[[Module]]).
                // 2. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
                // 3. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
                continue;
            };
            // iv. Else,
            // 1. Perform CreateImportBinding(env, in.[[LocalName]], resolution.[[Module]], resolution.[[BindingName]]).
            initialize_import_binding(
                envs,
                source_text_modules,
                env,
                r#in.local_name,
                module,
                binding_name,
            );
        }
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
        let environment = module
            .environment(agent)
            .expect("Attempted to execute unlinked module");
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
pub fn parse_module<'a>(
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
    let mut requested_modules = vec![];
    // 4. Let importEntries be the ImportEntries of body.
    let mut import_entries: Vec<ImportEntryRecord> = vec![];
    // 5. Let importedBoundNames be ImportedLocalNames(importEntries).
    let mut imported_bound_names = AHashSet::new();
    for ee in body.body.iter() {
        let Some(ee) = ee.as_module_declaration() else {
            continue;
        };
        match ee {
            ast::ModuleDeclaration::ImportDeclaration(ee) => {
                #[cfg(feature = "typescript")]
                if ee.import_kind.is_type() {
                    continue;
                }
                let module_request = ModuleRequestRecord::new(ee.source.value.as_str())
                    .unbind()
                    .bind(gc);
                requested_modules.push(module_request);
                let Some(specifiers) = &ee.specifiers else {
                    continue;
                };
                for specifier in specifiers {
                    match specifier {
                        ast::ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                            #[cfg(feature = "typescript")]
                            if specifier.import_kind.is_type() {
                                continue;
                            }
                            let imported = specifier.imported.name().as_str();
                            let import_name = String::from_str(agent, imported, gc);
                            let local_name = specifier.local.name.as_str();
                            imported_bound_names.insert(local_name);
                            let local_name = if imported == local_name {
                                import_name
                            } else {
                                String::from_str(agent, local_name, gc)
                            };
                            import_entries.push(ImportEntryRecord {
                                module_request,
                                import_name: Some(import_name),
                                local_name,
                            })
                        }
                        ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                            let local_name = specifier.local.name.as_str();
                            imported_bound_names.insert(local_name);
                            let local_name = String::from_str(agent, local_name, gc);
                            import_entries.push(ImportEntryRecord {
                                module_request,
                                import_name: Some(BUILTIN_STRING_MEMORY.default),
                                local_name,
                            })
                        }
                        ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                            let local_name = specifier.local.name.as_str();
                            imported_bound_names.insert(local_name);
                            let local_name = String::from_str(agent, local_name, gc);
                            import_entries.push(ImportEntryRecord {
                                module_request,
                                import_name: None,
                                local_name,
                            })
                        }
                    };
                }
            }
            ast::ModuleDeclaration::ExportAllDeclaration(_)
            | ast::ModuleDeclaration::ExportDefaultDeclaration(_)
            | ast::ModuleDeclaration::ExportNamedDeclaration(_) => {}
            ast::ModuleDeclaration::TSExportAssignment(_)
            | ast::ModuleDeclaration::TSNamespaceExportDeclaration(_) => unreachable!(),
        }
    }
    // 6. Let indirectExportEntries be a new empty List.
    let mut indirect_export_entries = vec![];
    // 7. Let localExportEntries be a new empty List.
    let mut local_export_entries = vec![];
    // 8. Let starExportEntries be a new empty List.
    let mut star_export_entries = vec![];
    // 9. Let exportEntries be the ExportEntries of body.
    // 10. For each ExportEntry Record ee of exportEntries, do
    for ee in body.body.iter() {
        let Some(ee) = ee.as_module_declaration() else {
            continue;
        };
        match ee {
            // a. If ee.[[ModuleRequest]] is null, then
            ast::ModuleDeclaration::ExportDefaultDeclaration(ee) => {
                match &ee.declaration {
                    ast::ExportDefaultDeclarationKind::FunctionDeclaration(ee) => {
                        // ExportDeclaration : export default HoistableDeclaration
                        // 1. Let names be the BoundNames of HoistableDeclaration.
                        // 2. Let localName be the sole element of names.
                        if let Some(local_name) = ee.id.as_ref() {
                            let local_name_str = local_name.name.as_str();
                            let local_name = String::from_str(agent, local_name_str, gc);
                            // i. If importedBoundNames does not contain ee.[[LocalName]], then
                            if !imported_bound_names.contains(local_name_str) {
                                // 1. Append ee to localExportEntries.
                                // 3. Return a List whose sole element is a new ExportEntry Record {
                                local_export_entries.push(LocalExportEntryRecord {
                                    // [[ExportName]]: "default"
                                    export_name: BUILTIN_STRING_MEMORY.default,
                                    // [[ModuleRequest]]: null,
                                    // [[ImportName]]: null,
                                    // [[LocalName]]: localName,
                                    local_name,
                                });
                                // }.
                            } else {
                                // ii. Else,
                                // 1. Let ie be the element of importEntries whose [[LocalName]] is ee.[[LocalName]].
                                let ie = import_entries
                                    .iter()
                                    .find(|element| element.local_name == local_name)
                                    .unwrap();
                                // 2. If ie.[[ImportName]] is not namespace-object, then
                                if let Some(import_name) = ie.import_name {
                                    // a. NOTE: This is a re-export of a single name.
                                    // b. Append the ExportEntry Record {
                                    indirect_export_entries.push(IndirectExportEntryRecord {
                                        // [[ModuleRequest]]: ie.[[ModuleRequest]],
                                        module_request: ie.module_request,
                                        // [[ImportName]]: ie.[[ImportName]],
                                        import_name: Some(import_name),
                                        // [[ExportName]]: ee.[[ExportName]]
                                        export_name: BUILTIN_STRING_MEMORY.default,
                                        // [[LocalName]]: null,
                                    });
                                    // } to indirectExportEntries.
                                } else {
                                    // 3. Else,
                                    // a. NOTE: This is a re-export of an imported module namespace object.
                                    // b. Append ee to localExportEntries.
                                    // 3. Return a List whose sole element is a new ExportEntry Record {
                                    local_export_entries.push(LocalExportEntryRecord {
                                        // [[ExportName]]: "default"
                                        export_name: BUILTIN_STRING_MEMORY.default,
                                        // [[ModuleRequest]]: null,
                                        // [[ImportName]]: null,
                                        // [[LocalName]]: localName,
                                        local_name,
                                    });
                                    // }.
                                    continue;
                                };
                            }
                        } else {
                            // Note: ImportedBoundNames never contains "*default*".
                            // 3. Return a List whose sole element is a new ExportEntry Record {
                            local_export_entries.push(LocalExportEntryRecord {
                                // [[ExportName]]: "default"
                                export_name: BUILTIN_STRING_MEMORY.default,
                                // [[ModuleRequest]]: null,
                                // [[ImportName]]: null,
                                // [[LocalName]]: localName,
                                local_name: BUILTIN_STRING_MEMORY._default_,
                            });
                            // }.
                        };
                    }
                    ast::ExportDefaultDeclarationKind::ClassDeclaration(ee) => {
                        // ExportDeclaration : export default ClassDeclaration
                        // 1. Let names be the BoundNames of ClassDeclaration.
                        // 2. Let localName be the sole element of names.
                        let local_name = ee
                            .id
                            .as_ref()
                            .map_or(BUILTIN_STRING_MEMORY._default_.bind(gc), |local_name| {
                                String::from_str(agent, local_name.name.as_str(), gc)
                            });
                        // 3. Return a List whose sole element is a new ExportEntry Record {
                        local_export_entries.push(LocalExportEntryRecord {
                            // [[ExportName]]: "default"
                            export_name: BUILTIN_STRING_MEMORY.default,
                            // [[ModuleRequest]]: null,
                            // [[ImportName]]: null,
                            // [[LocalName]]: localName,
                            local_name,
                        });
                        // }.
                    }
                    ast::ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => {}
                    _ => {
                        // ExportDeclaration : export default AssignmentExpression ;
                        // 1. Let entry be the ExportEntry Record {
                        local_export_entries.push(LocalExportEntryRecord {
                            // [[ModuleRequest]]: null,
                            // [[ImportName]]: null,
                            // [[LocalName]]: "*default*",
                            local_name: BUILTIN_STRING_MEMORY._default_,
                            // [[ExportName]]: "default"
                            export_name: BUILTIN_STRING_MEMORY.default,
                        });
                        // }.
                    }
                }
            }
            ast::ModuleDeclaration::ExportNamedDeclaration(ee) => {
                if let Some(source) = &ee.source {
                    // export { a, b as c } from "source";
                    //  ExportSpecifier : ModuleExportName
                    //  ExportSpecifier : ModuleExportName as ModuleExportName
                    let module_request = ModuleRequestRecord::new(&source.value);
                    debug_assert!(ee.declaration.is_none());
                    for entry in ee.specifiers.iter() {
                        // 1. Let sourceName be the StringValue of the first ModuleExportName.
                        let source_name = String::from_str(agent, entry.local.name().as_str(), gc);
                        // 2. Let exportName be the StringValue of the second ModuleExportName.
                        let export_name = if entry.local.name() == entry.exported.name() {
                            source_name
                        } else {
                            String::from_str(agent, entry.exported.name().as_str(), gc)
                        };
                        // a. Let localName be null.
                        // b. Let importName be sourceName.
                        let import_name = source_name;
                        // 4. Return a List whose sole element is a new ExportEntry Record {
                        indirect_export_entries.push(IndirectExportEntryRecord {
                            // [[ModuleRequest]]: module,
                            module_request,
                            // [[ImportName]]: importName,
                            import_name: Some(import_name),
                            // [[LocalName]]: localName,
                            // [[ExportName]]: export_name
                            export_name,
                        });
                        // }.
                    }
                } else {
                    // export { a, b as c };
                    // export var d;
                    // export let e;
                    // export const f;
                    // export function g() {}
                    // export class H {}
                    //  ExportDeclaration : export Declaration
                    //  ExportDeclaration : export VariableStatement
                    if let Some(decl) = &ee.declaration {
                        // 1. Let entries be a new empty List.
                        // 2. Let names be the BoundNames of Declaration.
                        // 2. Let names be the BoundNames of VariableStatement.
                        // 3. For each element name of names, do
                        decl.bound_names(&mut |name| {
                            let name = String::from_str(agent, name.name.as_str(), gc);
                            // a. Append the ExportEntry Record {
                            local_export_entries.push(LocalExportEntryRecord {
                                // [[ModuleRequest]]: null,
                                // [[ImportName]]: null,
                                // [[LocalName]]: name,
                                local_name: name,
                                // [[ExportName]]: name
                                export_name: name,
                            });
                            // } to entries.
                        });
                        // 4. Return entries.
                    } else {
                        for entry in ee.specifiers.iter() {
                            // 1. Let sourceName be the StringValue of the first ModuleExportName.
                            let source_name =
                                String::from_str(agent, entry.local.name().as_str(), gc);
                            // 2. Let exportName be the StringValue of the second ModuleExportName.
                            let export_name = if entry.local.name() == entry.exported.name() {
                                source_name
                            } else {
                                String::from_str(agent, entry.exported.name().as_str(), gc)
                            };
                            // a. Let localName be sourceName.
                            let local_name = source_name;
                            // b. Let importName be null.
                            // 4. Return a List whose sole element is a new ExportEntry Record {
                            local_export_entries.push(LocalExportEntryRecord {
                                // [[ModuleRequest]]: module,
                                // [[ImportName]]: importName,
                                // [[LocalName]]: localName,
                                local_name,
                                // [[ExportName]]: exportName
                                export_name,
                            });
                            // }.
                        }
                    }
                }
            }
            ast::ModuleDeclaration::ExportAllDeclaration(ee) => {
                if let Some(exported) = &ee.exported {
                    // c. Else,
                    // i. Append ee to indirectExportEntries.
                    // export * as ns from "foo";
                    indirect_export_entries.push(IndirectExportEntryRecord {
                        export_name: String::from_str(agent, exported.name().as_str(), gc),
                        module_request: ModuleRequestRecord::new(&ee.source.value),
                        import_name: None,
                    });
                } else {
                    // b. Else if ee.[[ImportName]] is all-but-default, then
                    // i. Assert: ee.[[ExportName]] is null.
                    // ii. Append ee to starExportEntries.
                    star_export_entries.push(ModuleRequestRecord::new(&ee.source.value));
                }
            }
            ast::ModuleDeclaration::ImportDeclaration(_) => {}
            ast::ModuleDeclaration::TSExportAssignment(_)
            | ast::ModuleDeclaration::TSNamespaceExportDeclaration(_) => unreachable!(),
        }
    }

    // SAFETY: The module requests refer to the string data of the module,
    // which lives at least as long as the SourceTextModuleRecord lives. Hence
    // we can (in an sketchy way) assert that their lifetime is "gc".
    let star_export_entries = star_export_entries.into_boxed_slice().unbind().bind(gc);
    let indirect_export_entries = indirect_export_entries.into_boxed_slice().unbind().bind(gc);

    // 11. Let async be body Contains await.
    let r#async = false;
    // 12. Return Source Text Module Record {
    Ok(agent.heap.create(SourceTextModuleRecord {
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
        cyclic_fields: CyclicModuleRecord::new(r#async, requested_modules.into_boxed_slice()),
        // [[ECMAScriptCode]]: body,
        ecmascript_code: ManuallyDrop::new(body),
        // [[Context]]: empty,
        context: Default::default(),
        // [[ImportMeta]]: empty,
        import_meta: Default::default(),
        // [[ImportEntries]]: importEntries,
        import_entries: import_entries.into_boxed_slice(),
        // [[LocalExportEntries]]: localExportEntries,
        local_export_entries: local_export_entries.into_boxed_slice(),
        // [[IndirectExportEntries]]: indirectExportEntries,
        indirect_export_entries,
        // [[StarExportEntries]]: starExportEntries,
        star_export_entries,

        source_code,
    }))
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

impl Rootable for SourceTextModule<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::SourceTextModule(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::SourceTextModule(object) => Some(object),
            _ => None,
        }
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
            import_entries,
            local_export_entries,
            indirect_export_entries,
            star_export_entries,
            source_code,
        } = self;
        abstract_fields.mark_values(queues);
        cyclic_fields.mark_values(queues);
        import_meta.mark_values(queues);
        source_code.mark_values(queues);
        import_entries.mark_values(queues);
        local_export_entries.mark_values(queues);
        indirect_export_entries.mark_values(queues);
        star_export_entries.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            abstract_fields,
            cyclic_fields,
            ecmascript_code: _,
            context: _,
            import_meta,
            import_entries,
            local_export_entries,
            indirect_export_entries,
            star_export_entries,
            source_code,
        } = self;
        abstract_fields.sweep_values(compactions);
        cyclic_fields.sweep_values(compactions);
        import_meta.sweep_values(compactions);
        source_code.sweep_values(compactions);
        import_entries.sweep_values(compactions);
        local_export_entries.sweep_values(compactions);
        indirect_export_entries.sweep_values(compactions);
        star_export_entries.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for ImportEntryRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            module_request,
            import_name,
            local_name,
        } = self;
        module_request.mark_values(queues);
        import_name.mark_values(queues);
        local_name.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            module_request,
            import_name,
            local_name,
        } = self;
        module_request.sweep_values(compactions);
        import_name.sweep_values(compactions);
        local_name.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for LocalExportEntryRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            export_name,
            local_name,
        } = self;
        export_name.mark_values(queues);
        local_name.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            export_name,
            local_name,
        } = self;
        export_name.sweep_values(compactions);
        local_name.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for IndirectExportEntryRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            export_name,
            module_request,
            import_name,
        } = self;
        export_name.mark_values(queues);
        module_request.mark_values(queues);
        import_name.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            export_name,
            module_request,
            import_name,
        } = self;
        export_name.sweep_values(compactions);
        module_request.sweep_values(compactions);
        import_name.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for ModuleRequestRecord<'static> {
    fn mark_values(&self, _queues: &mut WorkQueues) {
        let Self {
            specifier: _,
            attributes: _,
        } = self;
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        let Self {
            specifier: _,
            attributes: _,
        } = self;
    }
}

#[derive(Debug)]
pub(crate) struct SourceTextModuleHeap(pub(crate) Vec<SourceTextModuleRecord<'static>>);

impl Deref for SourceTextModuleHeap {
    type Target = Vec<SourceTextModuleRecord<'static>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SourceTextModuleHeap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<SourceTextModuleHeap> for SourceTextModuleHeap {
    fn as_ref(&self) -> &SourceTextModuleHeap {
        self
    }
}

impl AsMut<SourceTextModuleHeap> for SourceTextModuleHeap {
    fn as_mut(&mut self) -> &mut SourceTextModuleHeap {
        self
    }
}

impl AsRef<SourceTextModuleHeap> for Agent {
    fn as_ref(&self) -> &SourceTextModuleHeap {
        &self.heap.source_text_module_records
    }
}

impl AsMut<SourceTextModuleHeap> for Agent {
    fn as_mut(&mut self) -> &mut SourceTextModuleHeap {
        &mut self.heap.source_text_module_records
    }
}
