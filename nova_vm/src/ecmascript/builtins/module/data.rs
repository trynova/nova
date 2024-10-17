// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use small_string::SmallString;

use crate::{
    ecmascript::{
        execution::{ModuleEnvironmentIndex, RealmIdentifier},
        scripts_and_modules::module::ModuleIdentifier,
        types::{HeapString, OrdinaryObject, PropertyKey, String},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::Module;

#[derive(Debug, Clone)]
pub struct ModuleHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) module: ModuleRecord,
    pub(crate) exports: Box<[String]>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleRecord {
    /// \[\[Realm]]
    ///
    /// The Realm within which this module was created.
    realm: RealmIdentifier,
    /// \[\[Environment]]
    ///
    /// The Environment Record containing the top level bindings for this
    /// module. This field is set when the module is linked.
    pub(super) environment: Option<ModuleEnvironmentIndex>,
    /// \[\[Namespace]]
    ///
    /// The Module Namespace Object (28.3) if one has been created for this
    /// module.
    namespace: Option<Module>,
    /// \[\[HostDefined]]
    ///
    /// Field reserved for use by host environments that need to associate
    /// additional information with a module.
    host_defined: (),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolvedBindingName {
    String(HeapString),
    SmallString(SmallString),
    Namespace,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedBinding {
    /// \[\[Module]]
    pub(super) module: Option<ModuleIdentifier>,
    /// \[\[BindingName]]
    pub(super) binding_name: ResolvedBindingName,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolveExportResult {
    Ambiguous,
    Resolved(ResolvedBinding),
}

impl ModuleRecord {
    /// Return the binding of a name exported by this module. Bindings are
    /// represented by a ResolvedBinding Record, of the form { \[\[Module]]:
    /// Module Record, \[\[BindingName]]: String | NAMESPACE }. If the export
    /// is a Module Namespace Object without a direct binding in any module,
    /// \[\[BindingName]] will be set to NAMESPACE. Return null if the name
    /// cannot be resolved, or AMBIGUOUS if multiple bindings were found.
    ///
    /// Each time this operation is called with a specific exportName,
    /// resolveSet pair as arguments it must return the same result.
    ///
    /// LoadRequestedModules must have completed successfully prior to
    /// invoking this method.
    pub(crate) fn resolve_export(&self, _property_key: PropertyKey) -> Option<ResolveExportResult> {
        todo!()
    }
}

impl HeapMarkAndSweep for ModuleHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            module,
            exports,
        } = self;
        let ModuleRecord {
            realm,
            environment: _,
            namespace,
            host_defined: _,
        } = module;
        for ele in exports.iter() {
            ele.mark_values(queues);
        }
        realm.mark_values(queues);
        // environment.mark_values(queues);
        namespace.mark_values(queues);
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            module,
            exports,
        } = self;
        let ModuleRecord {
            realm,
            environment: _,
            namespace,
            host_defined: _,
        } = module;
        for ele in exports.iter_mut() {
            ele.sweep_values(compactions);
        }
        realm.sweep_values(compactions);
        // environment.sweep_values(compactions);
        namespace.sweep_values(compactions);
        object_index.sweep_values(compactions);
    }
}
