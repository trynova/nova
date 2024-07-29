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
pub struct ModuleHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    pub(crate) module: ModuleRecord<'gen>,
    pub(crate) exports: Box<[String<'gen>]>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleRecord<'gen> {
    /// \[\[Realm]]
    ///
    /// The Realm within which this module was created.
    realm: RealmIdentifier<'gen>,
    /// \[\[Environment]]
    ///
    /// The Environment Record containing the top level bindings for this
    /// module. This field is set when the module is linked.
    pub(super) environment: Option<ModuleEnvironmentIndex<'gen>>,
    /// \[\[Namespace]]
    ///
    /// The Module Namespace Object (28.3) if one has been created for this
    /// module.
    namespace: Option<Module<'gen>>,
    /// \[\[HostDefined]]
    ///
    /// Field reserved for use by host environments that need to associate
    /// additional information with a module.
    host_defined: (),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolvedBindingName<'gen> {
    String(HeapString<'gen>),
    SmallString(SmallString),
    Namespace,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedBinding<'gen> {
    /// \[\[Module]]
    pub(super) module: Option<ModuleIdentifier<'gen>>,
    /// \[\[BindingName]]
    pub(super) binding_name: ResolvedBindingName<'gen>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolveExportResult<'gen> {
    Ambiguous,
    Resolved(ResolvedBinding<'gen>),
}

impl<'gen> ModuleRecord<'gen> {
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
    pub(crate) fn resolve_export(&self, _property_key: PropertyKey<'gen>) -> Option<ResolveExportResult<'gen>> {
        todo!()
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for ModuleHeapData<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        for ele in self.exports.iter() {
            ele.mark_values(queues);
        }
        self.module.namespace.mark_values(queues);
        self.object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        for ele in self.exports.iter_mut() {
            ele.sweep_values(compactions);
        }
        self.module.namespace.sweep_values(compactions);
        self.object_index.sweep_values(compactions);
    }
}
