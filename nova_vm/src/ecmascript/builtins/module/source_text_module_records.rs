use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::{Atom, SourceType};
use small_string::SmallString;

use super::{abstract_module_records::{ModuleRecord, ResolveExportResult}, cyclic_module_records::{continue_module_loading, CyclicModuleRecord, CyclicModuleRecordStatus, LoadedModuleRecord}, data::ModuleHeapData, Module};
use crate::{
    ecmascript::{
        builtins::{control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability, module::{abstract_module_records::{ResolvedBinding, ResolvedBindingName}, cyclic_module_records::evaluate, semantics::{self, module_requests}}},
        execution::{Agent, DeclarativeEnvironmentIndex, ECMAScriptCodeEvaluationState, EnvironmentIndex, ExecutionContext, JsResult, RealmIdentifier},
        scripts_and_modules::{script::{HostDefined, ScriptIdentifier}, ScriptOrModule},
        types::{
            Object, String, BUILTIN_STRING_MEMORY, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT,
        },
    }, engine::{Executable, Vm}, heap::indexes::StringIndex
};

/// a String or NAMESPACE-OBJECT
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ImportName {
    String(StringIndex) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    NamespaceObject,
}

impl Into<ImportName> for String {
    fn into(self) -> ImportName {
        match self {
            String::String(data) => ImportName::String(data),
            String::SmallString(data) => ImportName::SmallString(data),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ImportEntryRecord {
    /// \[\[ModuleRequest\]\]
    ///
    /// a String
    ///
    /// String value of the ModuleSpecifier of the ImportDeclaration.
    ///
    /// SAFETY: The Atom refers to the Program that is owned by the
    /// ModuleHeapData that this ImportEntryRecord also is owned by.
    /// This is thus a self-referential struct and "safe".
    pub(crate) module_request: Atom<'static>,
    /// \[\[ImportName\]\]
    ///
    /// a String or NAMESPACE-OBJECT
    ///
    /// The name under which the desired binding is exported by the module
    /// identified by \[\[ModuleRequest\]\]. The value NAMESPACE-OBJECT indicates
    /// that the import request is for the target module's namespace object.
    pub(crate) import_name: ImportName,
    /// \[\[LocalName\]\]
    ///
    /// a String
    ///
    /// The name that is used to locally access the imported value from within the
    /// importing module.
    pub(crate) local_name: String,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
/// a String, all, or all-but-default
pub(crate) enum ExportImportName {
    String(StringIndex) = STRING_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    All,
    AllButDefault,
}

impl Into<ExportImportName> for String {
    fn into(self) -> ExportImportName {
        match self {
            String::String(data) => ExportImportName::String(data),
            String::SmallString(data) => ExportImportName::SmallString(data),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExportEntryRecord {
    /// \[\[ExportName\]\]
    ///
    /// a String or null
    ///
    /// The name used to export this binding by this module.
    pub(crate) export_name: Option<String>,
    /// \[\[ModuleRequest\]\]
    ///
    /// a String or null
    ///
    /// The String value of the ModuleSpecifier of the ExportDeclaration. null
    /// if the ExportDeclaration does not have a ModuleSpecifier.
    pub(crate) module_request: Option<Atom<'static>>,
    /// \[\[ImportName\]\]
    ///
    /// a String, null, all, or all-but-default
    ///
    /// The name under which the desired binding is exported by the module
    /// identified by \[\[ModuleRequest\]\]. null if the ExportDeclaration does
    /// not have a ModuleSpecifier. all is used for export * as ns from "mod"
    /// declarations. all-but-default is used for export * from "mod"
    /// declarations.
    pub(crate) import_name: Option<ExportImportName>,
    /// \[\[LocalName\]\]
    ///
    /// a String or null
    ///
    /// The name that is used to locally access the exported value from within
    /// the importing module. null if the exported value is not locally
    /// accessible from within the module.
    pub(crate) local_name: Option<String>,
}

#[derive(Debug)]
pub(crate) struct SourceTextModuleRecord {
    /// ### \[\[ECMAScriptCode\]\]
    ///
    /// a Parse Node
    ///
    /// The result of parsing the source text of this module using Module as
    /// the goal symbol.
    pub(crate) ecmascript_code: Program<'static>,
    source_text: Box<str>,
    /// ### \[\[Context\]\]
    ///
    /// an ECMAScript code execution context or empty
    ///
    /// The execution context associated with this module. It is empty until
    /// the module's environment has been initialized.
    context: Option<ExecutionContext>,
    /// ### \[\[ImportMeta\]\]
    ///
    /// an Object or empty
    ///
    /// An object exposed through the import.meta meta property. It is empty
    /// until it is accessed by ECMAScript code.
    import_meta: Option<Object>,
    /// ### \[\[ImportEntries\]\]
    ///
    /// a List of ImportEntry Records
    ///
    /// A List of ImportEntry records derived from the code of this module.
    import_entries: Box<[ImportEntryRecord]>,
    /// ### \[\[LocalExportEntries\]\]
    ///
    /// a List of ExportEntry Records
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to declarations that occur within the module.
    local_export_entries: Box<[ExportEntryRecord]>,
    /// ### \[\[IndirectExportEntries\]\]
    ///
    /// a List of ExportEntry Records
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to reexported imports that occur within the module or
    /// exports from export * as namespace declarations.
    indirect_export_entries: Box<[ExportEntryRecord]>,
    /// ### \[\[StarExportEntries\]\]
    ///
    /// a List of ExportEntry Records
    ///
    /// A List of ExportEntry records derived from the code of this module that
    /// correspond to export * declarations that occur within the module, not
    /// including export * as namespace declarations.
    star_export_entries: Box<[ExportEntryRecord]>,
}

unsafe impl Send for SourceTextModuleRecord {}

pub type ModuleOrErrors = Result<ModuleHeapData, Vec<oxc_diagnostics::Error>>;

/// ### [16.2.1.6.1 ParseModule ( sourceText, realm, hostDefined )]()
///
/// The abstract operation ParseModule takes arguments sourceText (ECMAScript
/// source text), realm (a Realm Record), and hostDefined (anything) and
/// returns a Source Text Module Record or a non-empty List of SyntaxError
/// objects. It creates a Source Text Module Record based upon the result of
/// parsing sourceText as a Module.
pub(crate) fn parse_module<'a>(
    agent: &mut Agent,
    allocator: &'static Allocator,
    source_text: Box<str>,
    realm: RealmIdentifier,
    host_defined: Option<HostDefined>,
) -> ModuleOrErrors {
    // 1. Let body be ParseText(sourceText, Module).
    let parser = Parser::new(
        allocator,
        // SAFETY: We're moving the Parser result into the same heap object as
        // source_text.
        unsafe { std::mem::transmute::<&str, &'static str>(&source_text) },
        SourceType::default().with_module(true),
    );
    let ParserReturn {
        errors, program, ..
    } = parser.parse();
    // 2. If body is a List of errors, return body.
    if !errors.is_empty() {
        return Err(errors);
    }
    // 3. Let requestedModules be the ModuleRequests of body.
    let requested_modules = module_requests(agent, &program);
    // 4. Let importEntries be ImportEntries of body.
    let import_entries = semantics::import_entries(agent, &program);
    // 5. Let importedBoundNames be ImportedLocalNames(importEntries).
    let imported_bound_names = semantics::imported_local_names(&import_entries);
    // 6. Let indirectExportEntries be a new empty List.
    let mut indirect_export_entries = vec![];
    // 7. Let localExportEntries be a new empty List.
    let mut local_export_entries = vec![];
    // 8. Let starExportEntries be a new empty List.
    let mut star_export_entries = vec![];
    // 9. Let exportEntries be ExportEntries of body.
    let mut export_entries = semantics::export_entries();
    // 10. For each ExportEntry Record ee of exportEntries, do
    for ee in export_entries.drain(..) {
        // a. If ee.[[ModuleRequest]] is null, then
        if ee.module_request.is_none() {
            let local_name = ee.local_name.unwrap();
            // i. If importedBoundNames does not contain ee.[[LocalName]], then
            if !imported_bound_names.contains(&local_name) {
                // 1. Append ee to localExportEntries.
                local_export_entries.push(ee);
            } else {
                // ii. Else,
                // 1. Let ie be the element of importEntries whose [[LocalName]] is ee.[[LocalName]].
                let ie = import_entries
                    .iter()
                    .find(|entry| entry.local_name == local_name)
                    .unwrap();
                // 2. If ie.[[ImportName]] is NAMESPACE-OBJECT, then
                if ie.import_name == ImportName::NamespaceObject {
                    // a. NOTE: This is a re-export of an imported module namespace object.
                    // b. Append ee to localExportEntries.
                    local_export_entries.push(ee);
                } else {
                    let import_name = match ie.import_name {
                        ImportName::String(data) => String::from(data),
                        ImportName::SmallString(data) => String::from(data),
                        _ => unreachable!(),
                    };
                    // 3. Else,
                    // a. NOTE: This is a re-export of a single name.
                    // b. Append the ExportEntry Record {
                    indirect_export_entries.push(ExportEntryRecord {
                        // [[ModuleRequest]]: ie.[[ModuleRequest]],
                        module_request: Some(ie.module_request.clone()),
                        // [[ImportName]]: ie.[[ImportName]],
                        import_name: Some(import_name.into()),
                        // [[LocalName]]: null,
                        local_name: None,
                        // [[ExportName]]: ee.[[ExportName]]
                        export_name: ee.export_name,
                    });
                    // } to indirectExportEntries.
                }
            }
        } else if ee.import_name != Some(BUILTIN_STRING_MEMORY.default.into()) {
            // b. Else if ee.[[ImportName]] is all-but-default, then
            // i. Assert: ee.[[ExportName]] is null.
            assert!(ee.export_name.is_none());
            // ii. Append ee to starExportEntries.
            star_export_entries.push(ee);
        } else {
            // c. Else,
            // i. Append ee to indirectExportEntries.
            indirect_export_entries.push(ee);
        }
    }
    // 11. Let async be body Contains await.
    let r#async = program
        .body
        .iter()
        .find(|statement| match statement {
            oxc_ast::ast::Statement::ExpressionStatement(expression) => match expression.expression
            {
                oxc_ast::ast::Expression::AwaitExpression(_) => true,
                _ => false,
            },
            _ => false,
        })
        .is_some();
    let requested_module_count = requested_modules.len();
    // 12. Return Source Text Module Record {
    Ok(ModuleHeapData {
        object_index: None,
        r#abstract: ModuleRecord {
            // [[Realm]]: realm,
            realm,
            // [[Environment]]: empty,
            environment: None,
            // [[Namespace]]: empty,
            namespace: false,
            // [[HostDefined]]: hostDefined,
            host_defined: host_defined,
        },
        cyclic: CyclicModuleRecord {
            // [[CycleRoot]]: empty,
            cycle_root: None,
            // [[HasTLA]]: async,
            has_top_level_await: r#async,
            // [[AsyncEvaluation]]: false,
            async_evaluation: false,
            // [[TopLevelCapability]]: empty,
            top_level_capability: None,
            // [[AsyncParentModules]]: « »,
            async_parent_modules: vec![],
            // [[PendingAsyncDependencies]]: empty,
            pending_async_dependencies: None,
            // [[Status]]: new,
            // [[EvaluationError]]: empty,
            // [[DFSIndex]]: empty,
            // [[DFSAncestorIndex]]: empty
            status: CyclicModuleRecordStatus::New,
            // [[RequestedModules]]: requestedModules,
            requested_modules,
            // [[LoadedModules]]: « »,
            loaded_modules: Vec::with_capacity(requested_module_count),
        },
        source_text: SourceTextModuleRecord {
            // [[ECMAScriptCode]]: body,
            ecmascript_code: program,
            source_text,
            // [[Context]]: empty,
            context: None,
            // [[ImportMeta]]: empty,
            import_meta: None,
            // [[ImportEntries]]: importEntries,
            import_entries,
            // [[LocalExportEntries]]: localExportEntries,
            local_export_entries: local_export_entries.into_boxed_slice(),
            // [[IndirectExportEntries]]: indirectExportEntries,
            indirect_export_entries: indirect_export_entries.into_boxed_slice(),
            // [[StarExportEntries]]: starExportEntries,
            star_export_entries: star_export_entries.into_boxed_slice(),
        },
        exports: Default::default(),
    })
    // }.
    // Note

    // An implementation may parse module source text and analyse it for Early
    // Error conditions prior to the evaluation of ParseModule for that module
    // source text. However, the reporting of any errors must be deferred until
    // the point where this specification actually performs ParseModule upon
    // that source text.
}

/// ### [16.2.1.6.2 GetExportedNames ( [ exportStarSet ] )]()
///
/// The GetExportedNames concrete method of a Source Text Module Record module
/// takes optional argument exportStarSet (a List of Source Text Module
/// Records) and returns a List of Strings.
pub(crate) fn get_exported_names(
    agent: &mut Agent,
    module: Module,
    export_start_set: Option<Vec<Module>>,
) -> Vec<String> {
    // 1. Assert: module.[[Status]] is not new.
    assert_ne!(agent[module].cyclic.status, CyclicModuleRecordStatus::New);
    // 2. If exportStarSet is not present, set exportStarSet to a new empty List.
    let mut export_start_set = export_start_set.unwrap_or(vec![]);
    // 3. If exportStarSet contains module, then
    if export_start_set.contains(&module) {
        // a. Assert: We've reached the starting point of an export * circularity.
        // b. Return a new empty List.
        return vec![];
    }
    // 4. Append module to exportStarSet.
    export_start_set.push(module);
    // 5. Let exportedNames be a new empty List.
    let mut exported_names = Vec::with_capacity(export_start_set.len());
    // 6. For each ExportEntry Record e of module.[[LocalExportEntries]], do
    for e in agent[module].source_text.local_export_entries.iter() {
        // a. Assert: module provides the direct binding for this export.
        // TODO: How to do this? Probably checking the environment?
        // b. Assert: e.[[ExportName]] is not null.
        // c. Append e.[[ExportName]] to exportedNames.
        exported_names.push(e.export_name.unwrap());
    }
    // 7. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
    for e in agent[module].source_text.indirect_export_entries.iter() {
        // a. Assert: module imports a specific binding for this export.
        // TODO: How?
        // b. Assert: e.[[ExportName]] is not null.
        // c. Append e.[[ExportName]] to exportedNames.
        exported_names.push(e.export_name.unwrap());
    }
    // 8. For each ExportEntry Record e of module.[[StarExportEntries]], do
    for e in agent[module].source_text.star_export_entries.clone().iter() {
        // a. Assert: e.[[ModuleRequest]] is not null.
        let module_request = e.module_request.clone().unwrap();
        // b. Let requestedModule be GetImportedModule(module, e.[[ModuleRequest]]).
        let requested_module = get_imported_module(agent, module, module_request);
        // c. Let starNames be requestedModule.GetExportedNames(exportStarSet).
        let star_names = requested_module.get_exported_names(agent, &mut export_start_set);
        // d. For each element n of starNames, do
        for &n in star_names.iter() {
            // i. If n is not "default", then
            // 1. If exportedNames does not contain n, then
            if n != BUILTIN_STRING_MEMORY.default && !exported_names.contains(&n) {
                // a. Append n to exportedNames.
                exported_names.push(n);
            }
        }
    }
    // 9. Return exportedNames.
    exported_names

    // Note

    // GetExportedNames does not filter out or throw an exception for names
    // that have ambiguous star export bindings.
}

#[derive(Debug, Clone)]
struct ResolveSetEntry {
    /// \[\[Module\]\]
    module: Module,
    /// \[\[ExportName\]\]    
    export_name: String,
}

/// ### [16.2.1.6.3 ResolveExport ( exportName [ , resolveSet ] )]()
///
/// The ResolveExport concrete method of a Source Text Module Record module
/// takes argument exportName (a String) and optional argument resolveSet (a
/// List of Records with fields \[\[Module\]\] (a Module Record) and
/// \[\[ExportName\]\] (a String)) and returns a ResolvedBinding Record,
/// null, or ambiguous.
///
///
/// ResolveExport attempts to resolve an imported binding to the actual
/// defining module and local binding name. The defining module may be the
/// module represented by the Module Record this method was invoked on or some
/// other module that is imported by that module. The parameter resolveSet is
/// used to detect unresolved circular import/export paths. If a pair
/// consisting of specific Module Record and exportName is reached that is
/// already in resolveSet, an import circularity has been encountered. Before
/// recursively calling ResolveExport, a pair consisting of module and
/// exportName is added to resolveSet.
///
/// If a defining module is found, a ResolvedBinding Record { \[\[Module\]\],
/// ### \[\[BindingName\]\] } is returned. This record identifies the resolved
/// binding of the originally requested export, unless this is the export of a
/// namespace with no local binding. In this case, \[\[BindingName\]\] will be
/// set to namespace. If no definition was found or the request is found to be
/// circular, null is returned. If the request is found to be ambiguous,
/// ambiguous is returned.
pub(crate) fn resolve_export(
    agent: &mut Agent,
    module: Module,
    export_name: String,
    resolve_set: Option<Vec<ResolveSetEntry>>,
) -> Option<ResolveExportResult> {
    // 1. Assert: module.[[Status]] is not new.
    assert!(agent[module].cyclic.status != CyclicModuleRecordStatus::New);
    // 2. If resolveSet is not present, set resolveSet to a new empty List.
    let mut resolve_set = resolve_set.unwrap_or(vec![]);
    // 3. For each Record { [[Module]], [[ExportName]] } r of resolveSet, do
    for r in &resolve_set {
        // a. If module and r.[[Module]] are the same Module Record and exportName is r.[[ExportName]], then
        if module == r.module && export_name == r.export_name {
            // i. Assert: This is a circular import request.
            // TODO: debug_assert!(module.requires_module(module));
            // ii. Return null.
            return None;
        }
    }
    // 4. Append the Record { [[Module]]: module, [[ExportName]]: exportName } to resolveSet.
    resolve_set.push(ResolveSetEntry {
        module,
        export_name,
    });
    // 5. For each ExportEntry Record e of module.[[LocalExportEntries]], do
    for e in agent[module].source_text.local_export_entries.iter() {
        // a. If e.[[ExportName]] is exportName, then
        if e.export_name == Some(export_name) {
            // i. Assert: module provides the direct binding for this export.
            let module_declarative_index = agent[module].r#abstract.environment.unwrap();
            let module_declarative_index =
                DeclarativeEnvironmentIndex::from_u32(module_declarative_index.into_u32());
            debug_assert!(agent
                .heap
                .environments
                .get_declarative_environment(module_declarative_index)
                .bindings
                .contains_key(&export_name));
            // ii. Return ResolvedBinding Record { [[Module]]: module, [[BindingName]]: e.[[LocalName]] }.
            return Some(ResolveExportResult::Resolved(ResolvedBinding {
                module: Some(module),
                binding_name: e.local_name.unwrap().into(),
            }));
        }
    }
    // 6. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
    for e in agent[module].source_text.indirect_export_entries.iter() {
        // a. If e.[[ExportName]] is exportName, then
        if e.export_name == Some(export_name) {
            // i. Assert: e.[[ModuleRequest]] is not null.
            let module_request = e.module_request.as_ref().unwrap().clone();
            // ii. Let importedModule be GetImportedModule(module, e.[[ModuleRequest]]).
            let imported_module = get_imported_module(agent, module, module_request);
            // iii. If e.[[ImportName]] is all, then
            if e.import_name == Some(ExportImportName::All) {
                // 1. Assert: module does not provide the direct binding for this export.
                let module_declarative_index = agent[module].r#abstract.environment.unwrap();
                let module_declarative_index =
                    DeclarativeEnvironmentIndex::from_u32(module_declarative_index.into_u32());
                debug_assert!(!agent
                    .heap
                    .environments
                    .get_declarative_environment(module_declarative_index)
                    .bindings
                    .contains_key(&export_name));
                // 2. Return ResolvedBinding Record { [[Module]]: importedModule, [[BindingName]]: namespace }.
                return Some(ResolveExportResult::Resolved(ResolvedBinding {
                    module: Some(imported_module),
                    binding_name: ResolvedBindingName::Namespace,
                }));
            } else {
                // iv. Else,
                // 1. Assert: module imports a specific binding for this export.
                // 2. Return importedModule.ResolveExport(e.[[ImportName]], resolveSet).
                let import_name = match e.import_name.unwrap() {
                    ExportImportName::String(d) => String::from(d),
                    ExportImportName::SmallString(d) => String::from(d),
                    _ => unreachable!(),
                };
                return resolve_export(agent, imported_module, import_name, Some(resolve_set));
            }
        }
    }
    // 7. If exportName is "default", then
    if export_name == BUILTIN_STRING_MEMORY.default {
        // a. Assert: A default export was not explicitly defined by this module.
        // TODO: Figure out what this meant again
        // b. Return null.
        return None;
        // c. NOTE: A default export cannot be provided by an export * from "mod" declaration.
    }
    // 8. Let starResolution be null.
    let mut star_resolution = None;
    // 9. For each ExportEntry Record e of module.[[StarExportEntries]], do
    for e in agent[module].source_text.star_export_entries.clone().iter() {
        // a. Assert: e.[[ModuleRequest]] is not null.
        let module_request = e.module_request.as_ref().unwrap().clone();
        // b. Let importedModule be GetImportedModule(module, e.[[ModuleRequest]]).
        let imported_module = get_imported_module(agent, module, module_request);
        // c. Let resolution be importedModule.ResolveExport(exportName, resolveSet).
        let resolution = resolve_export(
            agent,
            imported_module,
            export_name,
            Some(resolve_set.clone()),
        );
        match resolution {
            Some(resolution) => {
                match resolution {
                    ResolveExportResult::Ambiguous => {
                        // d. If resolution is ambiguous, return ambiguous.
                        return Some(ResolveExportResult::Ambiguous);
                    }
                    ResolveExportResult::Resolved(resolution) => {
                        // e. If resolution is not null, then
                        // i. Assert: resolution is a ResolvedBinding Record.
                        // ii. If starResolution is null, then
                        if star_resolution.is_none() {
                            // 1. Set starResolution to resolution.
                            star_resolution = Some(resolution);
                        } else {
                            let star_resolution = star_resolution.unwrap();
                            // iii. Else,
                            // 1. Assert: There is more than one * import that
                            // includes the requested name.
                            // 2. If resolution.[[Module]] and starResolution.[[Module]]
                            // are not the same Module Record, return ambiguous.
                            if resolution.module != star_resolution.module {
                                return Some(ResolveExportResult::Ambiguous);
                            }
                            // 3. If resolution.[[BindingName]] is not starResolution.[[BindingName]]
                            // and either resolution.[[BindingName]] or starResolution.[[BindingName]]
                            // is namespace, return ambiguous.
                            if resolution.binding_name != star_resolution.binding_name
                                && (resolution.binding_name == ResolvedBindingName::Namespace
                                    || star_resolution.binding_name
                                        == ResolvedBindingName::Namespace)
                            {
                                return Some(ResolveExportResult::Ambiguous);
                            }
                            // 4. If resolution.[[BindingName]] is a String, starResolution.[[BindingName]]
                            // is a String, and resolution.[[BindingName]] is not starResolution.[[BindingName]],
                            // return ambiguous.
                            if resolution.binding_name.is_string()
                                && star_resolution.binding_name.is_string()
                                && resolution.binding_name != star_resolution.binding_name
                            {
                                return Some(ResolveExportResult::Ambiguous);
                            }
                        }
                    }
                }
            }
            None => {}
        }
    }
    // 10. Return starResolution.
    star_resolution.map(|resolved_binding| ResolveExportResult::Resolved(resolved_binding))
}

/// ### [16.2.1.6.4 InitializeEnvironment ( )]()
///
/// The InitializeEnvironment concrete method of a Source Text Module Record
/// module takes no arguments and returns either a normal completion containing
/// unused or a throw completion.
pub(crate) fn initialize_environment(agent: &mut Agent, module: Module) -> JsResult<()> {
    // 1. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
    // a. Assert: e.[[ExportName]] is not null.
    // b. Let resolution be module.ResolveExport(e.[[ExportName]]).
    // c. If resolution is either null or ambiguous, throw a SyntaxError exception.
    // d. Assert: resolution is a ResolvedBinding Record.
    // 2. Assert: All named exports from module are resolvable.
    // 3. Let realm be module.[[Realm]].
    // 4. Assert: realm is not undefined.
    // 5. Let env be NewModuleEnvironment(realm.[[GlobalEnv]]).
    // 6. Set module.[[Environment]] to env.
    // 7. For each ImportEntry Record in of module.[[ImportEntries]], do
    // a. Let importedModule be GetImportedModule(module, in.[[ModuleRequest]]).
    // b. If in.[[ImportName]] is NAMESPACE-OBJECT, then
    // i. Let namespace be GetModuleNamespace(importedModule).
    // ii. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
    // iii. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
    // c. Else,
    // i. Let resolution be importedModule.ResolveExport(in.[[ImportName]]).
    // ii. If resolution is either null or ambiguous, throw a SyntaxError exception.
    // iii. If resolution.[[BindingName]] is namespace, then
    // 1. Let namespace be GetModuleNamespace(resolution.[[Module]]).
    // 2. Perform ! env.CreateImmutableBinding(in.[[LocalName]], true).
    // 3. Perform ! env.InitializeBinding(in.[[LocalName]], namespace).
    // iv. Else,
    // 1. Perform env.CreateImportBinding(in.[[LocalName]], resolution.[[Module]], resolution.[[BindingName]]).
    // 8. Let moduleContext be a new ECMAScript code execution context.
    // 9. Set the Function of moduleContext to null.
    // 10. Assert: module.[[Realm]] is not undefined.
    // 11. Set the Realm of moduleContext to module.[[Realm]].
    // 12. Set the ScriptOrModule of moduleContext to module.
    // 13. Set the VariableEnvironment of moduleContext to module.[[Environment]].
    // 14. Set the LexicalEnvironment of moduleContext to module.[[Environment]].
    // 15. Set the PrivateEnvironment of moduleContext to null.
    // 16. Set module.[[Context]] to moduleContext.
    // 17. Push moduleContext onto the execution context stack; moduleContext is now the running execution context.
    // 18. Let code be module.[[ECMAScriptCode]].
    // 19. Let varDeclarations be the VarScopedDeclarations of code.
    // 20. Let declaredVarNames be a new empty List.
    // 21. For each element d of varDeclarations, do
    // a. For each element dn of the BoundNames of d, do
    // i. If declaredVarNames does not contain dn, then
    // 1. Perform ! env.CreateMutableBinding(dn, false).
    // 2. Perform ! env.InitializeBinding(dn, undefined).
    // 3. Append dn to declaredVarNames.
    // 22. Let lexDeclarations be the LexicallyScopedDeclarations of code.
    // 23. Let privateEnv be null.
    // 24. For each element d of lexDeclarations, do
    // a. For each element dn of the BoundNames of d, do
    // i. If IsConstantDeclaration of d is true, then
    // 1. Perform ! env.CreateImmutableBinding(dn, true).
    // ii. Else,
    // 1. Perform ! env.CreateMutableBinding(dn, false).
    // iii. If d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration, then
    // 1. Let fo be InstantiateFunctionObject of d with arguments env and privateEnv.
    // 2. Perform ! env.InitializeBinding(dn, fo).
    // 25. Remove moduleContext from the execution context stack.
    // 26. Return unused.
    Ok(())
}

/// ### [16.2.1.6.5 ExecuteModule ( \[ capability \] )](https://tc39.es/ecma262/#sec-source-text-module-record-execute-module)
///
/// The ExecuteModule concrete method of a Source Text Module Record module
/// takes optional argument capability (a PromiseCapability Record) and returns
/// either a normal completion containing unused or a throw completion.
pub(crate) fn execute_module(
    agent: &mut Agent,
    module: Module,
    capability: Option<PromiseCapability>,
) -> JsResult<()> {
    // 1. Let moduleContext be a new ECMAScript code execution context.
    // 5. Assert: module has been linked and declarations in its module environment have been instantiated.
    // TODO: What else does this need to assert?
    let module_env = agent[module].r#abstract.environment.unwrap();
    let module_context = ExecutionContext {
        ecmascript_code: Some(ECMAScriptCodeEvaluationState {
            // 6. Set the VariableEnvironment of moduleContext to module.[[Environment]].
            // 7. Set the LexicalEnvironment of moduleContext to module.[[Environment]].
            lexical_environment: EnvironmentIndex::Module(module_env),
            variable_environment: EnvironmentIndex::Module(module_env),
            private_environment: None,
        }),
        // 2. Set the Function of moduleContext to null.
        function: None,
        // 3. Set the Realm of moduleContext to module.[[Realm]].
        realm: agent[module].r#abstract.realm,
        // 4. Set the ScriptOrModule of moduleContext to module.
        script_or_module: Some(ScriptOrModule::Module(module)),
    };
    // 8. Suspend the running execution context.
    // TODO: What does suspending mean again?

    // 9. If module.[[HasTLA]] is false, then
    if !agent[module].cyclic.has_top_level_await {
        // a. Assert: capability is not present.
        debug_assert!(capability.is_none());
        // b. Push moduleContext onto the execution context stack; moduleContext is now the running execution context.
        agent.execution_context_stack.push(module_context);
        // c. Let result be Completion(Evaluation of module.[[ECMAScriptCode]]).
        let exe = Executable::compile_module(agent, module);
        let result = Vm::execute(agent, &exe);
        // d. Suspend moduleContext and remove it from the execution context stack.
        agent.execution_context_stack.pop();
        // e. Resume the context that is now on the top of the execution context stack as the running execution context.
        // f. If result is an abrupt completion, then
        // i. Return ? result.
        result?;
    } else {
        // 10. Else,
        // a. Assert: capability is a PromiseCapability Record.
        let _capability = capability.unwrap();
        // b. Perform AsyncBlockStart(capability, module.[[ECMAScriptCode]], moduleContext).
        // async_block_start(agent, capability, agent[module].source_text.ecmascript_code, module_context);
        todo!("AsyncBlockStart");
    }
    // 11. Return unused.
    Ok(())
}

/// 16.2.1.7 GetImportedModule ( referrer, specifier )
///
/// The abstract operation GetImportedModule takes arguments referrer (a Cyclic Module Record) and specifier (a String) and returns a Module Record. It performs the following steps when called:
pub(super) fn get_imported_module(
    agent: &Agent,
    referrer: Module,
    specifier: Atom<'static>,
) -> Module {
    // 1. Assert: Exactly one element of referrer.[[LoadedModules]] is a Record
    // whose [[Specifier]] is specifier, since LoadRequestedModules has
    // completed successfully on referrer prior to invoking this abstract
    // operation.
    // 2. Let record be the Record in referrer.[[LoadedModules]] whose [[Specifier]] is specifier.
    let mut record: Option<Module> = None;
    assert_eq!(
        agent[referrer]
            .cyclic
            .loaded_modules
            .iter()
            .filter(|loaded_module| {
                if loaded_module.specifier == specifier {
                    record = Some(loaded_module.module);
                    true
                } else {
                    false
                }
            })
            .count(),
        1
    );
    // 3. Return record.[[Module]].
    record.unwrap()
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ModuleReferrer {
    Script(ScriptIdentifier),
    Module(Module),
    Realm(RealmIdentifier),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ModuleImportPayload {
    GraphLoadingState(Module),
    PromiseCapabilityRecord(PromiseCapability),
}

/// ### [16.2.1.9 FinishLoadingImportedModule ( referrer, specifier, payload, result )](https://tc39.es/ecma262/#sec-FinishLoadingImportedModule)
///
/// The abstract operation FinishLoadingImportedModule takes arguments referrer
/// (a Script Record, a Cyclic Module Record, or a Realm Record), specifier (a
/// String), payload (a GraphLoadingState Record or a PromiseCapability
/// Record), and result (either a normal completion containing a Module Record
/// or a throw completion) and returns unused.
pub(crate) fn finish_loading_imported_module(
    agent: &mut Agent,
    referrer: ModuleReferrer,
    specifier: Atom<'static>,
    payload: ModuleImportPayload,
    result: JsResult<Module>,
) {
    // 1. If result is a normal completion, then
    if let Ok(result) = result {
        let ModuleReferrer::Module(referrer) = referrer else {
            unreachable!("The spec seems to suggest that referrer can only be a Module?");
        };
        // a. If referrer.[[LoadedModules]] contains a Record whose [[Specifier]] is specifier, then
        if let Some(matching_module) = agent[referrer]
            .cyclic
            .loaded_modules
            .iter()
            .find(|loaded_module| loaded_module.specifier == Some(specifier))
        {
            // i. Assert: That Record's [[Module]] is result.[[Value]].
            assert_eq!(matching_module.module, result);
        } else {
            // b. Else,
            // i. Append the Record { [[Specifier]]: specifier, [[Module]]: result.[[Value]] } to referrer.[[LoadedModules]].
            agent[referrer]
                .cyclic
                .loaded_modules
                .push(LoadedModuleRecord {
                    specifier,
                    module: result,
                });
        }
    }
    // 2. If payload is a GraphLoadingState Record, then
    match payload {
        // a. Perform ContinueModuleLoading(payload, result).
        ModuleImportPayload::GraphLoadingState(payload) => {
            continue_module_loading(agent, payload, result)
        }
        // 3. Else,
        ModuleImportPayload::PromiseCapabilityRecord(payload) => {
            continue_dynamic_import(agent, payload, result)
        }
    }
    // a. Perform ContinueDynamicImport(payload, result).
    // 4. Return unused.
}

/// ### [16.2.1.10 GetModuleNamespace ( module )](https://tc39.es/ecma262/#sec-getmodulenamespace)
///
/// The abstract operation GetModuleNamespace takes argument module (an
/// instance of a concrete subclass of Module Record) and returns a Module
/// Namespace Object or empty. It retrieves the Module Namespace Object
/// representing module's exports, lazily creating it the first time it was
/// requested, and storing it in module.[[Namespace]] for future retrieval.
///
/// #### Note
///
/// GetModuleNamespace never throws. Instead, unresolvable names are simply
/// excluded from the namespace at this point. They will lead to a real
/// linking error later unless they are all ambiguous star exports that are
/// not explicitly requested anywhere.
pub(crate) fn get_module_namespace(agent: &mut Agent, module: Module) -> Module {
    // 1. Assert: If module is a Cyclic Module Record, then module.[[Status]] is not new or unlinked.
    debug_assert!({
        if true {
            !matches!(
                agent[module].cyclic.status,
                CyclicModuleRecordStatus::New | CyclicModuleRecordStatus::Unlinked
            )
        } else {
            true
        }
    });
    // 2. Let namespace be module.[[Namespace]].
    let namespace = agent[module].r#abstract.namespace;
    // 3. If namespace is empty, then
    if !namespace {
        // a. Let exportedNames be module.GetExportedNames().
        let exported_names = get_exported_names(agent, module, None);
        // b. Let unambiguousNames be a new empty List.
        let mut unamibigious_names = Vec::with_capacity(exported_names.len());
        // c. For each element name of exportedNames, do
        for name in exported_names {
            // i. Let resolution be module.ResolveExport(name).
            let resolution = resolve_export(agent, module, name, None);
            // ii. If resolution is a ResolvedBinding Record, append name to unambiguousNames.
            if let Some(ResolveExportResult::Resolved(_)) = resolution {
                unamibigious_names.push(name);
            }
        }
        // d. Set namespace to ModuleNamespaceCreate(module, unambiguousNames).
    }
    // 4. Return namespace.
    module
}
