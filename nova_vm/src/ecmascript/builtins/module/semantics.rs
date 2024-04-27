use std::ops::Deref;

use oxc_ast::ast::{ImportDeclarationSpecifier, Program};
use oxc_span::Atom;

use crate::ecmascript::{
    execution::Agent,
    types::{String, BUILTIN_STRING_MEMORY},
};

use super::source_text_module_records::{ExportEntryRecord, ImportEntryRecord, ImportName};

/// ###[16.2.1.2 Static Semantics: ImportedLocalNames ( importEntries )](https://tc39.es/ecma262/#sec-importedlocalnames)
///
/// The abstract operation ImportedLocalNames takes argument importEntries (a
/// List of ImportEntry Records) and returns a List of Strings. It creates a
/// List of all of the local name bindings defined by importEntries.
pub(crate) fn imported_local_names(import_entries: &[ImportEntryRecord]) -> Box<[String]> {
    // 1. Let localNames be a new empty List.
    // 2. For each ImportEntry Record i of importEntries, do
    import_entries
        .iter()
        .map(|i| {
            // a. Append i.[[LocalName]] to localNames.
            i.local_name
        })
        .collect()
    // 3. Return localNames.
}

/// ###[16.2.1.3 Static Semantics: ModuleRequests](https://tc39.es/ecma262/#sec-static-semantics-modulerequests)
///
/// The syntax-directed operation ModuleRequests takes no arguments and returns a List of Strings.
pub(crate) fn module_requests(agent: &mut Agent, program: &Program<'_>) -> Box<[String]> {
    let mut strings = vec![];
    // Module : [empty]
    // 1. Return a new empty List.
    for statement in program.body {
        // ModuleItemList : ModuleItem

        // 1. Return ModuleRequests of ModuleItem.
        // ModuleItemList : ModuleItemList ModuleItem

        // 1. Let moduleNames be ModuleRequests of ModuleItemList.
        // 2. Let additionalNames be ModuleRequests of ModuleItem.
        // 3. For each String name of additionalNames, do
        // a. If moduleNames does not contain name, then
        // i. Append name to moduleNames.
        // 4. Return moduleNames.
        match statement {
            oxc_ast::ast::Statement::ModuleDeclaration(decl) => match decl.deref() {
                // ImportDeclaration : import ImportClause FromClause ;
                oxc_ast::ast::ModuleDeclaration::ImportDeclaration(decl) => {
                    // 1. Return ModuleRequests of FromClause.
                    // ModuleSpecifier : StringLiteral
                    // 1. Return a List whose sole element is the SV of StringLiteral.
                    strings.push(String::from_str(agent, &decl.source.value));
                }
                oxc_ast::ast::ModuleDeclaration::ExportAllDeclaration(decl) => {
                    strings.push(String::from_str(agent, &decl.source.value));
                }
                oxc_ast::ast::ModuleDeclaration::ExportNamedDeclaration(decl) => {
                    // ExportDeclaration : export ExportFromClause FromClause ;
                    if let Some(source) = &decl.source {
                        // 1. Return the ModuleRequests of FromClause.
                        strings.push(String::from_str(agent, &source.value));
                    }
                }

                // ExportDeclaration :
                // export NamedExports ;
                // export VariableStatement
                // export Declaration
                // export default HoistableDeclaration
                // export default ClassDeclaration
                // export default AssignmentExpression ;
                // 1. Return a new empty List.
                _ => {}
            },
            // ModuleItem : StatementListItem
            // 1. Return a new empty List.
            _ => {}
        }
    }

    strings.into_boxed_slice()
}

/// 16.2.2.2 Static Semantics: ImportEntries
///
/// The syntax-directed operation ImportEntries takes no arguments and returns a List of ImportEntry Records.
pub fn import_entries(agent: &mut Agent, program: &Program<'_>) -> Box<[ImportEntryRecord]> {
    let mut entries = vec![];

    // Module : [empty]
    // 1. Return a new empty List.

    // ModuleItemList : ModuleItemList ModuleItem
    // 1. Let entries1 be ImportEntries of ModuleItemList.
    // 2. Let entries2 be ImportEntries of ModuleItem.
    // 3. Return the list-concatenation of entries1 and entries2.
    for statement in program.body {
        match statement {
            oxc_ast::ast::Statement::ModuleDeclaration(decl) => {
                match decl.deref() {
                    oxc_ast::ast::ModuleDeclaration::ImportDeclaration(decl) => {
                        // ImportDeclaration : import ModuleSpecifier ;
                        let Some(specifiers) = decl.specifiers else {
                            // 1. Return a new empty List.
                            continue;
                        };

                        // ImportDeclaration : import ImportClause FromClause ;

                        // 1. Let module be the sole element of ModuleRequests of FromClause.
                        // SAFETY: The Atom refers to the Program which will be moved to the
                        // Heap and will be owned by the ModuleHeapData that also owns the
                        // ImportEntryRecords. The Program's internal data is not moved, so the
                        // Atom references are "safe".
                        let module = unsafe {
                            std::mem::transmute::<Atom<'_>, Atom<'static>>(decl.source.value)
                        };
                        // 2. Return ImportEntriesForModule of ImportClause with argument module.
                        import_entries_for_module(agent, &specifiers, module, &mut |entry| {
                            entries.push(entry)
                        });
                    }
                    // ModuleItem :
                    // ExportDeclaration
                    // StatementListItem
                    // 1. Return a new empty List.
                    _ => {}
                }
            }
            _ => {}
        }
    }

    entries.into_boxed_slice()
}

/// 16.2.2.3 Static Semantics: ImportEntriesForModule
///
/// The syntax-directed operation ImportEntriesForModule takes argument module (a String) and returns a List of ImportEntry Records.
fn import_entries_for_module(
    agent: &mut Agent,
    specifiers: &[ImportDeclarationSpecifier<'_>],
    module: Atom<'static>,
    f: &mut impl FnMut(ImportEntryRecord),
) {
    // ImportClause : ImportedDefaultBinding , NameSpaceImport

    // 1. Let entries1 be ImportEntriesForModule of ImportedDefaultBinding with argument module.
    // 2. Let entries2 be ImportEntriesForModule of NameSpaceImport with argument module.
    // 3. Return the list-concatenation of entries1 and entries2.

    // ImportClause : ImportedDefaultBinding , NamedImports

    // 1. Let entries1 be ImportEntriesForModule of ImportedDefaultBinding with argument module.
    // 2. Let entries2 be ImportEntriesForModule of NamedImports with argument module.
    // 3. Return the list-concatenation of entries1 and entries2.

    // ImportsList : ImportsList , ImportSpecifier

    // 1. Let specs1 be the ImportEntriesForModule of ImportsList with argument module.
    // 2. Let specs2 be the ImportEntriesForModule of ImportSpecifier with argument module.
    // 3. Return the list-concatenation of specs1 and specs2.

    for specifier in specifiers {
        // NamedImports : { }
        // 1. Return a new empty List.
        match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                // ImportSpecifier : ImportedBinding
                // ImportSpecifier : ModuleExportName as ImportedBinding

                // 1. Let localName be the sole element of BoundNames of ImportedBinding.
                // 1. Let importName be the StringValue of ModuleExportName.
                // 2. Let localName be the StringValue of ImportedBinding.
                let local_name = String::from_str(agent, &specifier.local.name);

                // 2. Let entry be the ImportEntry Record {
                let entry = ImportEntryRecord {
                    // [[ModuleRequest]]: module,
                    module_request: specifier.local.name,
                    // [[ImportName]]: localName / importName,
                    import_name: BUILTIN_STRING_MEMORY.default.into(),
                    // [[LocalName]]: localName
                    local_name,
                };
                // 4. Return « entry ».
                f(entry);
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                // ImportedDefaultBinding : ImportedBinding

                // 1. Let localName be the sole element of BoundNames of ImportedBinding.
                let local_name = String::from_str(agent, &specifier.local.name);
                // 2. Let defaultEntry be the ImportEntry Record {
                let default_entry = ImportEntryRecord {
                    // [[ModuleRequest]]: module,
                    module_request: specifier.local.name,
                    // [[ImportName]]: "default",
                    import_name: BUILTIN_STRING_MEMORY.default.into(),
                    // [[LocalName]]: localName
                    local_name,
                };
                // }.
                // 3. Return « defaultEntry ».
                f(default_entry);
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                // NameSpaceImport : * as ImportedBinding

                // 1. Let localName be the StringValue of ImportedBinding.
                let local_name = String::from_str(agent, &specifier.local.name);
                // 2. Let entry be the ImportEntry Record {
                let entry = ImportEntryRecord {
                    // [[ModuleRequest]]: module,
                    module_request: specifier.local.name,
                    // [[ImportName]]: namespace-object,
                    import_name: ImportName::NamespaceObject,
                    // [[LocalName]]: localName
                    local_name,
                };
                // 3. Return « entry ».
                f(entry);
            }
        }
    }
}

/// ### [16.2.3.2 Static Semantics: ExportedBindings]()
///
/// The syntax-directed operation ExportedBindings takes no arguments and returns a List of Strings.
pub(crate) fn exported_bindings() -> Box<[String]> {
    let entries = vec![];
    // Note

    // ExportedBindings are the locally bound names that are explicitly associated with a Module's ExportedNames.

    // It is defined piecewise over the following productions:
    // ModuleItemList : ModuleItemList ModuleItem

    // 1. Let names1 be ExportedBindings of ModuleItemList.
    // 2. Let names2 be ExportedBindings of ModuleItem.
    // 3. Return the list-concatenation of names1 and names2.

    // ModuleItem :
    // ImportDeclaration
    // StatementListItem

    // 1. Return a new empty List.

    // ExportDeclaration :
    // export ExportFromClause FromClause ;

    // 1. Return a new empty List.

    // ExportDeclaration : export NamedExports ;

    // 1. Return the ExportedBindings of NamedExports.

    // ExportDeclaration : export VariableStatement

    // 1. Return the BoundNames of VariableStatement.

    // ExportDeclaration : export Declaration

    // 1. Return the BoundNames of Declaration.

    // ExportDeclaration :
    // export default HoistableDeclaration
    // export default ClassDeclaration
    // export default AssignmentExpression ;

    // 1. Return the BoundNames of this ExportDeclaration.

    // NamedExports : { }

    // 1. Return a new empty List.

    // ExportsList : ExportsList , ExportSpecifier

    // 1. Let names1 be the ExportedBindings of ExportsList.
    // 2. Let names2 be the ExportedBindings of ExportSpecifier.
    // 3. Return the list-concatenation of names1 and names2.

    // ExportSpecifier : ModuleExportName

    // 1. Return a List whose sole element is the StringValue of ModuleExportName.

    // ExportSpecifier : ModuleExportName as ModuleExportName

    // 1. Return a List whose sole element is the StringValue of the first ModuleExportName.
    entries.into_boxed_slice()
}

/// ### [16.2.3.3 Static Semantics: ExportedNames]()
///
/// The syntax-directed operation ExportedNames takes no arguments and returns a List of Strings.
pub(crate) fn exported_names() -> Box<[String]> {
    let entries = vec![];
    // Note

    // ExportedNames are the externally visible names that a Module explicitly maps to one of its local name bindings.

    // It is defined piecewise over the following productions:
    // ModuleItemList : ModuleItemList ModuleItem

    // 1. Let names1 be ExportedNames of ModuleItemList.
    // 2. Let names2 be ExportedNames of ModuleItem.
    // 3. Return the list-concatenation of names1 and names2.

    // ModuleItem : ExportDeclaration

    // 1. Return the ExportedNames of ExportDeclaration.

    // ModuleItem :
    // ImportDeclaration
    // StatementListItem

    // 1. Return a new empty List.

    // ExportDeclaration : export ExportFromClause FromClause ;

    // 1. Return the ExportedNames of ExportFromClause.

    // ExportFromClause : *

    // 1. Return a new empty List.

    // ExportFromClause : * as ModuleExportName

    // 1. Return a List whose sole element is the StringValue of ModuleExportName.

    // ExportFromClause : NamedExports

    // 1. Return the ExportedNames of NamedExports.

    // ExportDeclaration : export VariableStatement

    // 1. Return the BoundNames of VariableStatement.

    // ExportDeclaration : export Declaration

    // 1. Return the BoundNames of Declaration.

    // ExportDeclaration :
    // export default HoistableDeclaration
    // export default ClassDeclaration
    // export default AssignmentExpression ;

    // 1. Return « "default" ».

    // NamedExports : { }

    // 1. Return a new empty List.

    // ExportsList : ExportsList , ExportSpecifier

    // 1. Let names1 be the ExportedNames of ExportsList.
    // 2. Let names2 be the ExportedNames of ExportSpecifier.
    // 3. Return the list-concatenation of names1 and names2.

    // ExportSpecifier : ModuleExportName

    // 1. Return a List whose sole element is the StringValue of ModuleExportName.

    // ExportSpecifier : ModuleExportName as ModuleExportName

    // 1. Return a List whose sole element is the StringValue of the second ModuleExportName.
    entries.into_boxed_slice()
}

/// ### [16.2.3.4 Static Semantics: ExportEntries]()
///
/// The syntax-directed operation ExportEntries takes no arguments and returns a List of ExportEntry Records. It is defined piecewise over the following productions:
pub(crate) fn export_entries() -> Vec<ExportEntryRecord> {
    let entries = vec![];
    // Module : [empty]

    // 1. Return a new empty List.

    // ModuleItemList : ModuleItemList ModuleItem

    // 1. Let entries1 be ExportEntries of ModuleItemList.
    // 2. Let entries2 be ExportEntries of ModuleItem.
    // 3. Return the list-concatenation of entries1 and entries2.

    // ModuleItem :
    // ImportDeclaration
    // StatementListItem

    // 1. Return a new empty List.

    // ExportDeclaration : export ExportFromClause FromClause ;

    // 1. Let module be the sole element of ModuleRequests of FromClause.
    // 2. Return ExportEntriesForModule of ExportFromClause with argument module.

    // ExportDeclaration : export NamedExports ;

    // 1. Return ExportEntriesForModule of NamedExports with argument null.

    // ExportDeclaration : export VariableStatement

    // 1. Let entries be a new empty List.
    // 2. Let names be the BoundNames of VariableStatement.
    // 3. For each element name of names, do
    // a. Append the ExportEntry Record { [[ModuleRequest]]: null, [[ImportName]]: null, [[LocalName]]: name, [[ExportName]]: name } to entries.
    // 4. Return entries.

    // ExportDeclaration : export Declaration

    // 1. Let entries be a new empty List.
    // 2. Let names be the BoundNames of Declaration.
    // 3. For each element name of names, do
    // a. Append the ExportEntry Record { [[ModuleRequest]]: null, [[ImportName]]: null, [[LocalName]]: name, [[ExportName]]: name } to entries.
    // 4. Return entries.

    // ExportDeclaration : export default HoistableDeclaration

    // 1. Let names be BoundNames of HoistableDeclaration.
    // 2. Let localName be the sole element of names.
    // 3. Return a List whose sole element is a new ExportEntry Record { [[ModuleRequest]]: null, [[ImportName]]: null, [[LocalName]]: localName, [[ExportName]]: "default" }.

    // ExportDeclaration : export default ClassDeclaration

    // 1. Let names be BoundNames of ClassDeclaration.
    // 2. Let localName be the sole element of names.
    // 3. Return a List whose sole element is a new ExportEntry Record { [[ModuleRequest]]: null, [[ImportName]]: null, [[LocalName]]: localName, [[ExportName]]: "default" }.

    // ExportDeclaration : export default AssignmentExpression ;

    // 1. Let entry be the ExportEntry Record { [[ModuleRequest]]: null, [[ImportName]]: null, [[LocalName]]: "*default*", [[ExportName]]: "default" }.
    // 2. Return « entry ».

    // Note

    // "*default*" is used within this specification as a synthetic name for anonymous default export values. See this note for more details.
    entries
}

/// ### [16.2.3.5 Static Semantics: ExportEntriesForModule]()
///
/// The syntax-directed operation ExportEntriesForModule takes argument module (a String or null) and returns a List of ExportEntry Records. It is defined piecewise over the following productions:
pub(crate) fn export_entries_for_module() {
    // ExportFromClause : *

    // 1. Let entry be the ExportEntry Record { [[ModuleRequest]]: module, [[ImportName]]: all-but-default, [[LocalName]]: null, [[ExportName]]: null }.
    // 2. Return « entry ».

    // ExportFromClause : * as ModuleExportName

    // 1. Let exportName be the StringValue of ModuleExportName.
    // 2. Let entry be the ExportEntry Record { [[ModuleRequest]]: module, [[ImportName]]: all, [[LocalName]]: null, [[ExportName]]: exportName }.
    // 3. Return « entry ».

    // NamedExports : { }

    // 1. Return a new empty List.

    // ExportsList : ExportsList , ExportSpecifier

    // 1. Let specs1 be the ExportEntriesForModule of ExportsList with argument module.
    // 2. Let specs2 be the ExportEntriesForModule of ExportSpecifier with argument module.
    // 3. Return the list-concatenation of specs1 and specs2.

    // ExportSpecifier : ModuleExportName

    // 1. Let sourceName be the StringValue of ModuleExportName.
    // 2. If module is null, then
    // a. Let localName be sourceName.
    // b. Let importName be null.
    // 3. Else,
    // a. Let localName be null.
    // b. Let importName be sourceName.
    // 4. Return a List whose sole element is a new ExportEntry Record { [[ModuleRequest]]: module, [[ImportName]]: importName, [[LocalName]]: localName, [[ExportName]]: sourceName }.

    // ExportSpecifier : ModuleExportName as ModuleExportName

    // 1. Let sourceName be the StringValue of the first ModuleExportName.
    // 2. Let exportName be the StringValue of the second ModuleExportName.
    // 3. If module is null, then
    // a. Let localName be sourceName.
    // b. Let importName be null.
    // 4. Else,
    // a. Let localName be null.
    // b. Let importName be sourceName.
    // 5. Return a List whose sole element is a new ExportEntry Record { [[ModuleRequest]]: module, [[ImportName]]: importName, [[LocalName]]: localName, [[ExportName]]: exportName }.
}

/// ### [16.2.3.6 Static Semantics: ReferencedBindings]()
///
/// The syntax-directed operation ReferencedBindings takes no arguments and returns a List of Parse Nodes. It is defined piecewise over the following productions:
pub(crate) fn referenced_bindings() {
    // NamedExports : { }

    // 1. Return a new empty List.

    // ExportsList : ExportsList , ExportSpecifier

    // 1. Let names1 be the ReferencedBindings of ExportsList.
    // 2. Let names2 be the ReferencedBindings of ExportSpecifier.
    // 3. Return the list-concatenation of names1 and names2.

    // ExportSpecifier : ModuleExportName as ModuleExportName

    // 1. Return the ReferencedBindings of the first ModuleExportName.

    // ModuleExportName : IdentifierName

    // 1. Return a List whose sole element is the IdentifierName.

    // ModuleExportName : StringLiteral

    // 1. Return a List whose sole element is the StringLiteral.
}
