// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [16.2.1 Module Semantics](https://tc39.es/ecma262/#sec-module-semantics)

use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use abstract_module_records::{
    AbstractModule, AbstractModuleMethods, AbstractModuleSlots, ResolvedBinding,
};
use ahash::AHasher;
use cyclic_module_records::{
    CyclicModuleRecordStatus, CyclicModuleSlots, GraphLoadingStateRecord, continue_module_loading,
};
use hashbrown::{HashTable, hash_table::Entry};
use oxc_ast::ast;
use source_text_module_records::SourceTextModule;

use crate::{
    ecmascript::{
        builtins::module::{Module, module_namespace_create},
        execution::{Agent, JsResult, Realm},
        scripts_and_modules::{
            ScriptOrModule,
            script::{HostDefined, Script},
        },
        types::String,
    },
    engine::{
        context::{Bindable, GcToken, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::continue_dynamic_import;
pub mod abstract_module_records;
pub mod cyclic_module_records;
pub mod source_text_module_records;

/// ### [16.2.1.3 ModuleRequest Records](https://tc39.es/ecma262/#sec-modulerequest-record)
///
/// A ModuleRequest Record represents the request to import a module with given
/// import attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleRequestRecord<'a> {
    /// ### \[\[Specifier]]
    ///
    /// a String
    ///
    /// The module specifier
    specifier: String<'a>,
    /// ### \[\[Attributes]]
    ///
    /// a List of ImportAttribute Records
    ///
    /// The import attributes
    ///
    /// > NOTE: The attributes are sorted by key in a stable, deterministic,
    /// > but undetermined order.
    attributes: Option<Box<[ImportAttributeRecord<'a>]>>,
    /// Precomputed hash of the ModuleRequest specifier and attributes.
    hash: u64,
}

impl<'a> ModuleRequestRecord<'a> {
    /// ### \[\[Attributes]]
    fn attributes(&self) -> &[ImportAttributeRecord<'a>] {
        self.attributes.as_ref().map_or(&[], |attrs| attrs.as_ref())
    }
}

impl AsRef<[ModuleRequestRecord<'static>]> for Agent {
    fn as_ref(&self) -> &[ModuleRequestRecord<'static>] {
        &self.heap.module_request_records
    }
}

impl AsMut<[ModuleRequestRecord<'static>]> for Agent {
    fn as_mut(&mut self) -> &mut [ModuleRequestRecord<'static>] {
        &mut self.heap.module_request_records
    }
}

/// ### [16.2.1.3 ModuleRequest Records](https://tc39.es/ecma262/#sec-modulerequest-record)
///
/// A ModuleRequest Record represents the request to import a module with given
/// import attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ModuleRequest<'a>(u32, PhantomData<&'a GcToken>);

impl<'r> ModuleRequest<'r> {
    /// Create a new ModuleRequest from a specifier string and a `with`
    /// clause.
    pub(super) fn new(
        agent: &mut Agent,
        specifier: &str,
        with_clause: Option<&ast::WithClause>,
        gc: NoGcScope<'r, '_>,
    ) -> Self {
        let mut state = AHasher::default();
        specifier.hash(&mut state);
        let specifier = String::from_str(agent, specifier, gc).unbind();
        let attributes = with_clause.map(|with_clause| {
            // Note: we first have to collect the key-value pairs into a
            // separate boxed slice to be able to sort them by key. Only then
            // can we hash them and perform the conversion into
            // ImportAttributeRecord structs.
            let mut key_value_pairs = with_clause
                .with_entries
                .iter()
                .map(|attr| {
                    let key = attr.key.as_atom().as_str();
                    let value = attr.value.value.as_str();
                    (key, value)
                })
                .collect::<Box<[(&str, &str)]>>();
            key_value_pairs.sort_by_key(|attr| attr.0);

            key_value_pairs
                .into_iter()
                .map(|(key, value)| {
                    key.hash(&mut state);
                    value.hash(&mut state);
                    ImportAttributeRecord {
                        key: String::from_str(agent, key, gc).unbind(),
                        value: String::from_str(agent, value, gc).unbind(),
                    }
                })
                .collect()
        });
        let hash = state.finish();
        let index = agent.heap.module_request_records.len() as u32;
        agent.heap.module_request_records.push(ModuleRequestRecord {
            specifier,
            attributes,
            hash,
        });
        Self(index, PhantomData)
    }

    /// Create a new ModuleRequest from a specifier String and a list of
    /// import attribute records.
    pub(super) fn new_dynamic(
        agent: &mut Agent,
        specifier: String,
        attributes: Vec<ImportAttributeRecord>,
        gc: NoGcScope<'r, '_>,
    ) -> Self {
        let mut state = AHasher::default();
        let attributes = attributes.into_boxed_slice();
        specifier.to_string_lossy(agent).hash(&mut state);
        for attribute in attributes.iter() {
            attribute.key.to_string_lossy(agent).hash(&mut state);
            attribute.value.to_string_lossy(agent).hash(&mut state);
        }
        let hash = state.finish();
        let index = agent.heap.module_request_records.len() as u32;
        agent.heap.module_request_records.push(
            ModuleRequestRecord {
                specifier,
                attributes: Some(attributes),
                hash,
            }
            .unbind(),
        );
        Self(index, PhantomData).bind(gc)
    }

    pub(crate) fn get_index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn get<'a>(
        self,
        agent: &'a [ModuleRequestRecord<'static>],
    ) -> &'a ModuleRequestRecord<'static> {
        &agent[self.get_index()]
    }

    /// Get the ModuleRequest's \[\[Specifier]] string.
    pub fn specifier(self, agent: &Agent) -> String<'r> {
        self.get(agent.as_ref()).specifier
    }

    pub fn attributes(self, agent: &Agent) -> &[ImportAttributeRecord<'r>] {
        self.get(agent.as_ref()).attributes()
    }
}

bindable_handle!(ModuleRequestRecord);

bindable_handle!(ModuleRequest);

/// # [LoadedModuleRequest Records](https://tc39.es/ecma262/#table-loadedmodulerequest-fields)
#[derive(Debug)]
pub(crate) struct LoadedModuleRequestRecord<'a> {
    module_request: ModuleRequest<'a>,
    /// ### \[\[Module]]
    ///
    /// a Module Record
    ///
    /// The loaded module corresponding to this module request
    module: AbstractModule<'a>,
}

/// # [ImportAttribute Records](https://tc39.es/ecma262/#table-importattribute-fields)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAttributeRecord<'a> {
    /// ### \[\[Key]]
    ///
    /// a String
    ///
    /// The attribute key
    key: String<'a>,
    /// ### \[\[Value]]
    ///
    /// a String
    ///
    /// The attribute value
    value: String<'a>,
}

bindable_handle!(ImportAttributeRecord);

/// ### \[\[LoadedModules]]
///
/// a List of LoadedModuleRequest Records
///
/// A map from the module requests to loaded used by the module represented by this
/// record to request the importation of a module with the relative import
/// attributes to the resolved Module Record. The list does not contain two
/// different Records r1 and r2 such that ModuleRequestsEqual(r1, r2) is true.
#[derive(Debug, Default)]
pub(crate) struct LoadedModules<'a> {
    table: HashTable<LoadedModuleRequestRecord<'a>>,
}

impl<'a> LoadedModules<'a> {
    /// Get a loaded module for a module request, if present.
    pub(crate) fn get_loaded_module(
        &self,
        requests: &[ModuleRequestRecord<'static>],
        module_request: ModuleRequest<'a>,
    ) -> Option<AbstractModule<'a>> {
        let hash = module_request.get(requests).hash;
        self.table
            .find(hash, |record| {
                module_requests_equal(
                    record.module_request.get(requests),
                    module_request.get(requests),
                )
            })
            .map(|record| record.module)
    }

    /// Add a loaded module for a module request.
    pub(crate) fn insert_loaded_module<'gc>(
        &mut self,
        requests: &[ModuleRequestRecord<'static>],
        module_request: ModuleRequest<'gc>,
        module: AbstractModule<'gc>,
    ) {
        let hash = module_request.get(requests).hash;
        // a. If referrer.[[LoadedModules]] contains a LoadedModuleRequest
        //    Record record such that ModuleRequestsEqual(record,
        //    moduleRequest) is true, then
        match self.table.entry(
            hash,
            |record| {
                module_requests_equal(
                    record.module_request.get(requests),
                    module_request.get(requests),
                )
            },
            |record| record.module_request.get(requests).hash,
        ) {
            Entry::Occupied(e) => {
                // i. Assert: record.[[Module]] and result.[[Value]] are the
                //    same Module Record.
                assert!(e.get().module == module);
            }
            Entry::Vacant(e) => {
                // b. Else,
                // i. Append the LoadedModuleRequest Record {
                e.insert(LoadedModuleRequestRecord {
                    // [[Specifier]]: moduleRequest.[[Specifier]],
                    // [[Attributes]]: moduleRequest.[[Attributes]],
                    module_request: module_request.unbind(),
                    // [[Module]]: result.[[Value]]
                    module: module.unbind(),
                });
                // } to referrer.[[LoadedModules]].
            }
        }
    }
}

/// ### [16.2.1.3.1 ModuleRequestsEqual ( left, right )](https://tc39.es/ecma262/#sec-ModuleRequestsEqual)
///
/// The abstract operation ModuleRequestsEqual takes arguments left (a
/// ModuleRequest Record or a LoadedModuleRequest Record) and right (a
/// ModuleRequest Record or a LoadedModuleRequest Record) and returns a
/// Boolean.
fn module_requests_equal(left: &ModuleRequestRecord, right: &ModuleRequestRecord) -> bool {
    // 1. If left.[[Specifier]] is not right.[[Specifier]], return false.
    if left.specifier != right.specifier {
        return false;
    }
    // 2. Let leftAttrs be left.[[Attributes]].
    let left_attrs = left.attributes();
    // 3. Let rightAttrs be right.[[Attributes]].
    let right_attrs = right.attributes();
    // 4. Let leftAttrsCount be the number of elements in leftAttrs.
    let left_attrs_count = left_attrs.len();
    // 5. Let rightAttrsCount be the number of elements in rightAttrs.
    let right_attrs_count = right_attrs.len();
    // 6. If leftAttrsCount ≠ rightAttrsCount, return false.
    if left_attrs_count != right_attrs_count {
        return false;
    }
    // NOTE: ModuleRequestRecords have an invariant over module imports holding
    // attributes ordered by key.
    // 7. For each ImportAttribute Record l of leftAttrs, do
    for (l, r) in left_attrs.iter().zip(right_attrs.iter()) {
        // a. If rightAttrs does not contain an ImportAttribute Record r
        //    such that l.[[Key]] is r.[[Key]] and l.[[Value]] is
        //    r.[[Value]], return false.
        if l.key != r.key || l.value != r.value {
            return false;
        }
    }
    // 8. Return true.
    true
}

/// ### [16.2.1.9 GetImportedModule ( referrer, request )](https://tc39.es/ecma262/#sec-GetImportedModule)
///
/// The abstract operation GetImportedModule takes arguments referrer (a Cyclic
/// Module Record) and request (a ModuleRequest Record) and returns a Module
/// Record.
fn get_imported_module<'a>(
    agent: &Agent,
    referrer: SourceTextModule<'a>,
    request: ModuleRequest,
    gc: NoGcScope<'a, '_>,
) -> AbstractModule<'a> {
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

/// Module loading referrer.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Referrer<'a>(InnerReferrer<'a>);

impl Referrer<'_> {
    /// Get the Realm that this referrer belongs to.
    pub fn realm<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Realm<'a> {
        match self.0 {
            InnerReferrer::Script(s) => s.realm(agent, gc),
            InnerReferrer::SourceTextModule(m) => m.realm(agent, gc),
            InnerReferrer::Realm(r) => r.bind(gc),
        }
    }

    /// Get the host defined data of this referrer.
    pub fn host_defined(self, agent: &Agent) -> Option<HostDefined> {
        match self.0 {
            InnerReferrer::Script(s) => s.host_defined(agent),
            InnerReferrer::SourceTextModule(m) => m.host_defined(agent),
            InnerReferrer::Realm(r) => r.host_defined(agent),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum InnerReferrer<'a> {
    Script(Script<'a>),
    SourceTextModule(SourceTextModule<'a>),
    Realm(Realm<'a>),
}

impl<'a> From<Script<'a>> for Referrer<'a> {
    fn from(value: Script<'a>) -> Self {
        Self(InnerReferrer::Script(value))
    }
}

impl<'a> From<SourceTextModule<'a>> for Referrer<'a> {
    fn from(value: SourceTextModule<'a>) -> Self {
        Self(InnerReferrer::SourceTextModule(value))
    }
}

impl<'a> From<Realm<'a>> for Referrer<'a> {
    fn from(value: Realm<'a>) -> Self {
        Self(InnerReferrer::Realm(value))
    }
}

impl<'a> From<ScriptOrModule<'a>> for Referrer<'a> {
    fn from(value: ScriptOrModule<'a>) -> Self {
        match value {
            ScriptOrModule::Script(s) => Self(InnerReferrer::Script(s)),
            ScriptOrModule::SourceTextModule(m) => Self(InnerReferrer::SourceTextModule(m)),
        }
    }
}

bindable_handle!(Referrer);

impl Rootable for Referrer<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        InnerReferrer::to_root_repr(value.0)
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        InnerReferrer::from_root_repr(value).map(Self)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        InnerReferrer::from_heap_data(heap_data).map(Self)
    }
}

bindable_handle!(InnerReferrer);

impl Rootable for InnerReferrer<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::Script(s) => Err(HeapRootData::Script(s.unbind())),
            Self::SourceTextModule(m) => Err(HeapRootData::SourceTextModule(m.unbind())),
            Self::Realm(r) => Err(HeapRootData::Realm(r.unbind())),
        }
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Script(s) => Some(Self::Script(s)),
            HeapRootData::SourceTextModule(m) => Some(Self::SourceTextModule(m)),
            HeapRootData::Realm(r) => Some(Self::Realm(r)),
            _ => None,
        }
    }
}

impl Referrer<'_> {
    fn insert_loaded_module(
        self,
        agent: &mut Agent,
        request: ModuleRequest,
        module: AbstractModule,
    ) {
        match self.0 {
            InnerReferrer::Script(s) => s.insert_loaded_module(agent, request, module),
            InnerReferrer::SourceTextModule(m) => m.insert_loaded_module(agent, request, module),
            InnerReferrer::Realm(r) => r.insert_loaded_module(agent, request, module),
        }
    }
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
    referrer: Referrer<'a>,
    module_request: ModuleRequest<'a>,
    payload: &mut GraphLoadingStateRecord<'a>,
    result: JsResult<'a, AbstractModule<'a>>,
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
    if payload.pending_modules_count > 0 {
        // HACK: [[PendingModulesCount]] is being used as a marker between
        // static and dynamic import. Static import always has non-zero count.
        // a. Perform ContinueModuleLoading(payload, result).
        continue_module_loading(agent, payload, result, gc);
    } else {
        // 3. Else,
        // a. Perform ContinueDynamicImport(payload, result).
        continue_dynamic_import(agent, payload.promise_capability.clone(), result, gc);
    }
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
    module: AbstractModule,
    gc: NoGcScope<'a, '_>,
) -> Module<'a> {
    let module = module.bind(gc);
    if let Some(module) = module.as_source_text_module() {
        // 1. Assert: If module is a Cyclic Module Record, then module.[[Status]]
        //    is not new or unlinked.
        debug_assert!(!matches!(
            module.status(agent),
            CyclicModuleRecordStatus::New | CyclicModuleRecordStatus::Unlinked
        ));
    }
    // 2. Let namespace be module.[[Namespace]].
    let namespace = module.namespace(agent, gc);
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
                let resolution = module.resolve_export(agent, *name, &mut vec![], gc);
                // ii. If resolution is a ResolvedBinding Record, append name to
                //     unambiguousNames.
                matches!(resolution, Some(ResolvedBinding::Resolved { .. }))
            })
            .collect::<Box<[String]>>();
        // d. Set namespace to ModuleNamespaceCreate(module, unambiguousNames).
        return module_namespace_create(agent, module, unambiguous_names, gc);
    };
    // 4. Return namespace.
    namespace
}

impl HeapMarkAndSweep for ModuleRequest<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.module_request_records.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .module_request_records
            .shift_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for LoadedModules<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        for loaded_module_request in self.table.iter() {
            loaded_module_request.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        // Note: we do not need to perform rehashing of the table, as we've
        // hashed the requests based on their string data which doesn't change
        // during GC, unlike the String indexes.
        for loaded_module_request in self.table.iter_mut() {
            loaded_module_request.sweep_values(compactions);
        }
    }
}

impl HeapMarkAndSweep for LoadedModuleRequestRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            module_request,
            module,
        } = self;
        module_request.mark_values(queues);
        module.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            module_request,
            module,
        } = self;
        module_request.sweep_values(compactions);
        module.sweep_values(compactions);
    }
}
