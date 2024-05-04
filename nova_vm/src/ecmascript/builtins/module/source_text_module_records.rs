use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::{Atom, SourceType};
use oxc_syntax::module_record::RequestedModule;
use small_string::SmallString;

use super::{abstract_module_records::{ModuleRecord, ResolveExportResult}, cyclic_module_records::{CyclicModuleRecord, CyclicModuleRecordStatus}, data::ModuleHeapData, Module};
use crate::{
    ecmascript::{
        builtins::{control_abstraction_objects::promise_objects::promise_abstract_operations::promise_capability_records::PromiseCapability, module::semantics::{self, module_requests}},
        execution::{Agent, ExecutionContext, JsResult, RealmIdentifier},
        scripts_and_modules::script::HostDefined,
        types::{
            Object, String, BUILTIN_STRING_MEMORY, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT,
        },
    },
    heap::indexes::StringIndex,
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

pub type ModuleOrErrors = Result<ModuleHeapData, Vec<oxc_diagnostics::Error>>;

/// ### [16.2.1.6.1 ParseModule ( sourceText, realm, hostDefined )]()
///
/// The abstract operation ParseModule takes arguments sourceText (ECMAScript
/// source text), realm (a Realm Record), and hostDefined (anything) and
/// returns a Source Text Module Record or a non-empty List of SyntaxError
/// objects. It creates a Source Text Module Record based upon the result of
/// parsing sourceText as a Module.
pub(crate) fn parse_module(
    agent: &mut Agent,
    allocator: &Allocator,
    module: Module,
    source_text: Box<str>,
    realm: RealmIdentifier,
    host_defined: Option<HostDefined>,
) -> ModuleOrErrors {
    // 1. Let body be ParseText(sourceText, Module).
    let parser = Parser::new(
        allocator,
        &source_text,
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
                        module_request: Some(ie.module_request),
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
    // 12. Return Source Text Module Record {
    Ok(ModuleHeapData {
        
        object_index: None,
        r#abstract: ModuleRecord {
            // [[Realm]]: realm,
            realm,
            // [[Environment]]: empty,
            environment: None,
            // [[Namespace]]: empty,
            namespace: None,
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
            loaded_modules: vec![],
        },
        source_text: SourceTextModuleRecord {
            // [[ECMAScriptCode]]: body,
            ecmascript_code: program,
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
        exports: todo!(),
    })
    // }.
    // Note

    // An implementation may parse module source text and analyse it for Early Error conditions prior to the evaluation of ParseModule for that module source text. However, the reporting of any errors must be deferred until the point where this specification actually performs ParseModule upon that source text.
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
    for e in agent[module].source_text.star_export_entries.iter() {
        // a. Assert: e.[[ModuleRequest]] is not null.
        let module_request = e.module_request.unwrap();
        // b. Let requestedModule be GetImportedModule(module, e.[[ModuleRequest]]).
        let requested_module = get_imported_module(agent, module, module_request);
        // c. Let starNames be requestedModule.GetExportedNames(exportStarSet).
        let star_names = requested_module.get_exported_names(agent, export_start_set);
        // d. For each element n of starNames, do
        for n in star_names.iter() {
            // i. If n is not "default", then
            if n != BUILTIN_STRING_MEMORY.default && 
            // 1. If exportedNames does not contain n, then
                !exported_names.contains(n) {
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

/// ### [16.2.1.6.3 ResolveExport ( exportName [ , resolveSet ] )]()
///
/// The ResolveExport concrete method of a Source Text Module Record module
/// takes argument exportName (a String) and optional argument resolveSet (a
/// List of Records with fields \[\[Module\]\] (a Module Record) and
/// ### \[\[ExportName\]\] (a String)) and returns a ResolvedBinding Record, null,
///
/// or ambiguous.
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
///
/// binding of the originally requested export, unless this is the export of a
///
/// namespace with no local binding. In this case, \[\[BindingName\]\] will be
/// set to namespace. If no definition was found or the request is found to be
/// circular, null is returned. If the request is found to be ambiguous,
/// ambiguous is returned.
pub(crate) fn resolve_export(
    agent: &mut Agent,
    module: Module,
    export_name: String,
    resolve_set: Option<()>,
) -> ResolveExportResult {
    // 1. Assert: module.[[Status]] is not new.
    // 2. If resolveSet is not present, set resolveSet to a new empty List.
    // 3. For each Record { [[Module]], [[ExportName]] } r of resolveSet, do
    // a. If module and r.[[Module]] are the same Module Record and exportName is r.[[ExportName]], then
    // i. Assert: This is a circular import request.
    // ii. Return null.
    // 4. Append the Record { [[Module]]: module, [[ExportName]]: exportName } to resolveSet.
    // 5. For each ExportEntry Record e of module.[[LocalExportEntries]], do
    // a. If e.[[ExportName]] is exportName, then
    // i. Assert: module provides the direct binding for this export.
    // ii. Return ResolvedBinding Record { [[Module]]: module, [[BindingName]]: e.[[LocalName]] }.
    // 6. For each ExportEntry Record e of module.[[IndirectExportEntries]], do
    // a. If e.[[ExportName]] is exportName, then
    // i. Assert: e.[[ModuleRequest]] is not null.
    // ii. Let importedModule be GetImportedModule(module, e.[[ModuleRequest]]).
    // iii. If e.[[ImportName]] is all, then
    // 1. Assert: module does not provide the direct binding for this export.
    // 2. Return ResolvedBinding Record { [[Module]]: importedModule, [[BindingName]]: namespace }.
    // iv. Else,
    // 1. Assert: module imports a specific binding for this export.
    // 2. Return importedModule.ResolveExport(e.[[ImportName]], resolveSet).
    // 7. If exportName is "default", then
    // a. Assert: A default export was not explicitly defined by this module.
    // b. Return null.
    // c. NOTE: A default export cannot be provided by an export * from "mod" declaration.
    // 8. Let starResolution be null.
    // 9. For each ExportEntry Record e of module.[[StarExportEntries]], do
    // a. Assert: e.[[ModuleRequest]] is not null.
    // b. Let importedModule be GetImportedModule(module, e.[[ModuleRequest]]).
    // c. Let resolution be importedModule.ResolveExport(exportName, resolveSet).
    // d. If resolution is ambiguous, return ambiguous.
    // e. If resolution is not null, then
    // i. Assert: resolution is a ResolvedBinding Record.
    // ii. If starResolution is null, then
    // 1. Set starResolution to resolution.
    // iii. Else,
    // 1. Assert: There is more than one * import that includes the requested name.
    // 2. If resolution.[[Module]] and starResolution.[[Module]] are not the same Module Record, return ambiguous.
    // 3. If resolution.[[BindingName]] is not starResolution.[[BindingName]] and either resolution.[[BindingName]] or starResolution.[[BindingName]] is namespace, return ambiguous.
    // 4. If resolution.[[BindingName]] is a String, starResolution.[[BindingName]] is a String, and resolution.[[BindingName]] is not starResolution.[[BindingName]], return ambiguous.
    // 10. Return starResolution.
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
    // 2. Set the Function of moduleContext to null.
    // 3. Set the Realm of moduleContext to module.[[Realm]].
    // 4. Set the ScriptOrModule of moduleContext to module.
    // 5. Assert: module has been linked and declarations in its module environment have been instantiated.
    // 6. Set the VariableEnvironment of moduleContext to module.[[Environment]].
    // 7. Set the LexicalEnvironment of moduleContext to module.[[Environment]].
    // 8. Suspend the running execution context.
    // 9. If module.[[HasTLA]] is false, then
    // a. Assert: capability is not present.
    // b. Push moduleContext onto the execution context stack; moduleContext is now the running execution context.
    // c. Let result be Completion(Evaluation of module.[[ECMAScriptCode]]).
    // d. Suspend moduleContext and remove it from the execution context stack.
    // e. Resume the context that is now on the top of the execution context stack as the running execution context.
    // f. If result is an abrupt completion, then
    // i. Return ? result.
    // 10. Else,
    // a. Assert: capability is a PromiseCapability Record.
    // b. Perform AsyncBlockStart(capability, module.[[ECMAScriptCode]], moduleContext).
    // 11. Return unused.
    Ok(())
}


/// 16.2.1.7 GetImportedModule ( referrer, specifier )
///
/// The abstract operation GetImportedModule takes arguments referrer (a Cyclic Module Record) and specifier (a String) and returns a Module Record. It performs the following steps when called:
pub(super) fn get_imported_module(agent: &mut Agent, referrer: Module, specifier: Atom<'static>) -> Module {
    // 1. Assert: Exactly one element of referrer.[[LoadedModules]] is a Record
    // whose [[Specifier]] is specifier, since LoadRequestedModules has
    // completed successfully on referrer prior to invoking this abstract
    // operation.
    // 2. Let record be the Record in referrer.[[LoadedModules]] whose [[Specifier]] is specifier.
    let mut record: Option<Module> = None;
    assert_eq!(agent[referrer].cyclic.loaded_modules.iter().filter(|loaded_module| {
        if loaded_module.specifier == specifier {
            record = Some(loaded_module.module);
            true
        } else {
            false
        }
    }).count(), 1);
    // 3. Return record.[[Module]].
    record.unwrap()
}


