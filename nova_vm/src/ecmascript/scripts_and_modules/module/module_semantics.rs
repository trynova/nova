// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2.1 Module Semantics](https://tc39.es/ecma262/#sec-module-semantics)

use abstract_module_records::{ModuleAbstractMethods, ResolvedBinding};
use cyclic_module_records::{
    CyclicModuleRecordStatus, GraphLoadingStateRecord, continue_module_loading,
};
use source_text_module_records::{SourceTextModule, SourceTextModuleHeap};

use crate::{
    ecmascript::{
        builtins::module::{Module, module_namespace_create},
        execution::{Agent, JsResult},
        types::String,
    },
    engine::context::{Bindable, NoGcScope},
};
pub mod abstract_module_records;
pub mod cyclic_module_records;
pub mod source_text_module_records;

/// ### [16.2.1.3 ModuleRequest Records](https://tc39.es/ecma262/#sec-modulerequest-record)
///
/// A ModuleRequest Record represents the request to import a module with given
/// import attributes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct ModuleRequestRecord<'a> {
    /// ### \[\[Specifier]]
    ///
    /// a String
    ///
    /// The module specifier
    specifier: &'a str,
    /// ### \[\[Attributes]]
    ///
    /// a List of ImportAttribute Records
    ///
    /// The import attributes
    attributes: (),
}

impl<'a> ModuleRequestRecord<'a> {
    /// Create a new ModuleRequestRecord from an Atom
    fn new(specifier: &'a str) -> Self {
        Self {
            specifier,
            attributes: (),
        }
    }

    /// ### \[\[Specifier]]
    pub fn specifier(&self) -> &str {
        self.specifier
    }

    /// ### \[\[Attributes]]
    pub fn attributes(&self) -> &() {
        &self.attributes
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ModuleRequestRecord<'_> {
    type Of<'a> = ModuleRequestRecord<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<_, _>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<_, _>(self) }
    }
}

/// ### [16.2.1.9 GetImportedModule ( referrer, request )](https://tc39.es/ecma262/#sec-GetImportedModule)
///
/// The abstract operation GetImportedModule takes arguments referrer (a Cyclic
/// Module Record) and request (a ModuleRequest Record) and returns a Module
/// Record.
fn get_imported_module<'a>(
    agent: &impl AsRef<SourceTextModuleHeap>,
    referrer: SourceTextModule<'a>,
    request: &ModuleRequestRecord,
    gc: NoGcScope<'a, '_>,
) -> SourceTextModule<'a> {
    // 1. Let records be a List consisting of each LoadedModuleRequest Record r
    //    of referrer.[[LoadedModules]] such that ModuleRequestsEqual(r,
    //    request) is true.
    // 2. Assert: records has exactly one element, since LoadRequestedModules
    //    has completed successfully on referrer prior to invoking this
    //    abstract operation.
    // 3. Let record be the sole element of records.
    // 4. Return record.[[Module]].
    referrer
        .get_loaded_module(agent, request)
        .expect("Could not find loaded module for request")
        .bind(gc)
}

/// ### [16.2.1.11 FinishLoadingImportedModule ( referrer, moduleRequest, payload, result )](https://tc39.es/ecma262/#sec-FinishLoadingImportedModule)
///
/// The abstract operation FinishLoadingImportedModule takes arguments referrer
/// (a Script Record, a Cyclic Module Record, or a Realm Record), moduleRequest
/// (a ModuleRequest Record), payload (a GraphLoadingState Record or a
/// PromiseCapability Record), and result (either a normal completion
/// containing a Module Record or a throw completion) and returns unused.
pub fn finish_loading_imported_module<'a>(
    agent: &mut Agent,
    referrer: SourceTextModule<'a>,
    module_request: &'a ModuleRequestRecord<'a>,
    payload: &mut GraphLoadingStateRecord<'a>,
    result: JsResult<'a, SourceTextModule<'a>>,
    gc: NoGcScope<'a, '_>,
) {
    // 1. If result is a normal completion, then
    if let Ok(result) = result {
        // a. If referrer.[[LoadedModules]] contains a LoadedModuleRequest
        //    Record record such that ModuleRequestsEqual(record,
        //    moduleRequest) is true, then
        // i. Assert: record.[[Module]] and result.[[Value]] are the same
        //    Module Record.
        // b. Else,
        // i. Append the LoadedModuleRequest Record {
        // [[Specifier]]: moduleRequest.[[Specifier]],
        // [[Attributes]]: moduleRequest.[[Attributes]],
        // [[Module]]: result.[[Value]]
        // } to referrer.[[LoadedModules]].
        referrer.insert_loaded_module(agent, module_request, result);
    }
    // 2. If payload is a GraphLoadingState Record, then
    // a. Perform ContinueModuleLoading(payload, result).
    continue_module_loading(agent, payload, result, gc);
    // 3. Else,
    // a. Perform ContinueDynamicImport(payload, result).
    // 4. Return unused.
}

/// ### [16.2.1.13 GetModuleNamespace ( module )](https://tc39.es/ecma262/#sec-getmodulenamespace)
///
/// The abstract operation GetModuleNamespace takes argument module (an
/// instance of a concrete subclass of Module Record) and returns a Module
/// Namespace Object. It retrieves the Module Namespace Object representing
/// module's exports, lazily creating it the first time it was requested, and
/// storing it in module.\[\[Namespace]] for future retrieval.
///
/// > NOTE: GetModuleNamespace never throws. Instead, unresolvable names are
/// > simply excluded from the namespace at this point. They will lead to a
/// > real linking error later unless they are all ambiguous star exports that
/// > are not explicitly requested anywhere.
pub(crate) fn get_module_namespace<'a>(
    agent: &mut Agent,
    module: SourceTextModule,
    gc: NoGcScope<'a, '_>,
) -> Module<'a> {
    let module = module.bind(gc);
    // 1. Assert: If module is a Cyclic Module Record, then module.[[Status]]
    //    is not new or unlinked.
    debug_assert!(!matches!(
        module.status(agent),
        CyclicModuleRecordStatus::New | CyclicModuleRecordStatus::Unlinked
    ));
    // 2. Let namespace be module.[[Namespace]].
    let namespace = module.namespace(agent);
    // 3. If namespace is empty, then
    let Some(namespace) = namespace else {
        // a. Let exportedNames be module.GetExportedNames().
        let exported_names = module.get_exported_names(agent, &mut vec![], gc);
        // b. Let unambiguousNames be a new empty List.
        // c. For each element name of exportedNames, do
        let unambiguous_names = exported_names
            .into_iter()
            .filter(|name| {
                // i. Let resolution be module.ResolveExport(name).
                let resolution = module.resolve_export(agent, *name, None, gc);
                // ii. If resolution is a ResolvedBinding Record, append name to
                //     unambiguousNames.
                matches!(resolution, Some(ResolvedBinding::Resolved { .. }))
            })
            .collect::<Box<[String]>>();
        // d. Set namespace to ModuleNamespaceCreate(module, unambiguousNames).
        return module_namespace_create(agent, module, unambiguous_names);
    };
    // 4. Return namespace.
    namespace
}
