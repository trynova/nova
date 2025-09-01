// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [16.2.1.5 Abstract Module Records](https://tc39.es/ecma262/#sec-abstract-module-records)

use crate::{
    ecmascript::{
        builtins::{module::Module, promise::Promise},
        execution::{Agent, JsResult, ModuleEnvironment, Realm},
        scripts_and_modules::script::HostDefined,
        types::String,
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::source_text_module_records::SourceTextModule;

/// ### [16.2.1.5 Abstract Module Records](https://tc39.es/ecma262/#sec-abstract-module-records)
#[derive(Debug)]
pub(crate) struct AbstractModuleRecord<'a> {
    /// ### \[\[Realm]]
    ///
    /// The Realm within which this module was created.
    realm: Realm<'a>,
    /// ### \[\[Environment]]
    ///
    /// The Environment Record containing the top level bindings for this
    /// module. This field is set when the module is linked.
    environment: Option<ModuleEnvironment<'a>>,
    /// ### \[\[Namespace]]
    ///
    /// The Module Namespace Object (28.3) if one has been created for this
    /// module.
    namespace: Option<Module<'a>>,
    /// ### \[\[HostDefined]]
    ///
    /// Field reserved for use by host environments that need to associate
    /// additional information with a module.
    host_defined: Option<HostDefined>,
}

unsafe impl Send for AbstractModuleRecord<'_> {}

impl<'m> AbstractModuleRecord<'m> {
    pub(super) fn new(realm: Realm<'m>, host_defined: Option<HostDefined>) -> Self {
        Self {
            realm,
            environment: None,
            namespace: None,
            host_defined,
        }
    }

    /// ### \[\[Environment]]
    pub(super) fn environment(&self) -> Option<ModuleEnvironment<'m>> {
        self.environment
    }

    /// Set \[\[Environment]] to env.
    pub(super) fn set_environment(&mut self, env: ModuleEnvironment) {
        assert!(
            self.environment.replace(env.unbind()).is_none(),
            "Attempted to set module environment twice"
        );
    }

    /// ### \[\[Namespace]]
    pub(super) fn namespace(&self) -> Option<Module<'m>> {
        self.namespace
    }

    /// Set \[\[Namespace]] to namespace.
    pub(super) fn set_namespace(&mut self, namespace: Module) {
        assert!(
            self.namespace.replace(namespace.unbind()).is_none(),
            "Attempted to set module namespace twice"
        );
    }

    /// ### \[\[Realm]]
    pub(super) fn realm(&self) -> Realm<'m> {
        self.realm
    }

    /// ### \[\[\HostDefined]]
    pub(crate) fn host_defined(&self) -> Option<HostDefined> {
        self.host_defined.clone()
    }
}

/// ### [16.2.1.5 Abstract Module Records](https://tc39.es/ecma262/#sec-abstract-module-records)
///
/// A Module Record encapsulates structural information about the imports and
/// exports of a single module. This information is used to link the imports
/// and exports of sets of connected modules. A Module Record includes four
/// fields that are only used when evaluating a module.
///
/// For specification purposes Module Record values are values of the Record
/// specification type and can be thought of as existing in a simple
/// object-oriented hierarchy where Module Record is an abstract class with
/// both abstract and concrete subclasses. This specification defines the
/// abstract subclass named Cyclic Module Record and its concrete subclass
/// named Source Text Module Record. Other specifications and implementations
/// may define additional Module Record subclasses corresponding to alternative
/// module definition facilities that they defined.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct AbstractModule<'a>(InnerAbstractModule<'a>);

impl core::fmt::Debug for AbstractModule<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.fmt(f),
        }
    }
}

impl<'m> AbstractModule<'m> {
    #[inline]
    pub(super) fn as_source_text_module(self) -> Option<SourceTextModule<'m>> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => Some(m),
        }
    }
}

impl<'a> From<SourceTextModule<'a>> for AbstractModule<'a> {
    fn from(value: SourceTextModule<'a>) -> Self {
        Self(InnerAbstractModule::SourceTextModule(value))
    }
}

// SAFETY: Pass-through
unsafe impl Bindable for AbstractModule<'_> {
    type Of<'a> = AbstractModule<'a>;

    fn unbind(self) -> Self::Of<'static> {
        AbstractModule(self.0.unbind())
    }

    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        AbstractModule(self.0.bind(gc))
    }
}

impl Rootable for AbstractModule<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        InnerAbstractModule::to_root_repr(value.0)
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        InnerAbstractModule::from_root_repr(value).map(Self)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        InnerAbstractModule::from_heap_data(heap_data).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) enum InnerAbstractModule<'a> {
    SourceTextModule(SourceTextModule<'a>),
}

// SAFETY: Pass-through.
unsafe impl Bindable for InnerAbstractModule<'_> {
    type Of<'a> = InnerAbstractModule<'a>;

    fn unbind(self) -> Self::Of<'static> {
        match self {
            Self::SourceTextModule(m) => InnerAbstractModule::SourceTextModule(m.unbind()),
        }
    }

    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        match self {
            Self::SourceTextModule(m) => InnerAbstractModule::SourceTextModule(m.bind(gc)),
        }
    }
}

impl Rootable for InnerAbstractModule<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Self::SourceTextModule(m) => Err(HeapRootData::SourceTextModule(m.unbind())),
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
            HeapRootData::SourceTextModule(m) => Some(Self::SourceTextModule(m)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResolvedBinding<'a> {
    Ambiguous,
    Resolved {
        /// \[\[Module]]
        module: AbstractModule<'a>,
        /// \[\[BindingName]]
        binding_name: Option<String<'a>>,
    },
}

pub(crate) trait AbstractModuleSlots: Copy {
    /// ### \[\[Environment]]
    fn environment<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>)
    -> Option<ModuleEnvironment<'a>>;

    /// Set \[\[Environment]] to env.
    fn set_environment(self, agent: &mut Agent, env: ModuleEnvironment);

    /// ### \[\[Namespace]]
    fn namespace<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Option<Module<'a>>;

    /// Set \[\[Namespace]] to namespace.
    fn set_namespace(self, agent: &mut Agent, namespace: Module);

    /// ### \[\[Realm]]
    fn realm<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Realm<'a>;

    /// ### \[\[HostDefined]]
    fn host_defined(self, agent: &Agent) -> Option<HostDefined>;
}

pub(crate) struct ResolveSetEntry<'a> {
    /// ### \[\[Module]]
    pub(crate) module: SourceTextModule<'a>,
    /// ### \[\[ExportName]]
    pub(crate) export_name: String<'a>,
}

/// ### [Abstract Methods of Module Records](https://tc39.es/ecma262/#table-abstract-methods-of-module-records)
pub(crate) trait AbstractModuleMethods: AbstractModuleSlots {
    /// ### LoadRequestedModules(\[hostDefined])
    ///
    /// Prepares the module for linking by recursively loading all its
    /// dependencies, and returns a promise.
    #[must_use]
    fn load_requested_modules<'a>(
        self,
        agent: &mut Agent,
        host_defined: Option<HostDefined>,
        gc: NoGcScope<'a, '_>,
    ) -> Promise<'a>;

    /// ### GetExportedNames(\[exportStarSet])
    ///
    /// Return a list of all names that are either directly or indirectly
    /// exported from this module.
    ///
    /// LoadRequestedModules must have completed successfully prior to invoking
    /// this method.
    fn get_exported_names<'a>(
        self,
        agent: &Agent,
        export_start_set: &mut Vec<SourceTextModule<'a>>,
        gc: NoGcScope<'a, '_>,
    ) -> Vec<String<'a>>;

    /// ### ResolveExport(exportName \[, resolveSet])
    ///
    /// Return the binding of a name exported by this module. Bindings are
    /// represented by a ResolvedBinding Record, of the form `{ [[Module]]:
    /// Module Record, [[BindingName]]: String | namespace }`. If the export is
    /// a Module Namespace Object without a direct binding in any module,
    /// `[[BindingName]]` will be set to namespace. Return null if the name
    /// cannot be resolved, or ambiguous if multiple bindings were found.
    ///
    /// Each time this operation is called with a specific exportName,
    /// resolveSet pair as arguments it must return the same result.
    ///
    /// LoadRequestedModules must have completed successfully prior to invoking
    /// this method.
    fn resolve_export<'a>(
        self,
        agent: &Agent,
        export_name: String,
        resolve_set: &mut Vec<ResolveSetEntry<'a>>,
        gc: NoGcScope<'a, '_>,
    ) -> Option<ResolvedBinding<'a>>;

    /// ### Link()
    ///
    /// Prepare the module for evaluation by transitively resolving all module
    /// dependencies and creating a Module Environment Record.
    ///
    /// LoadRequestedModules must have completed successfully prior to invoking
    /// this method.
    fn link<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> JsResult<'a, ()>;

    /// ### Evaluate()
    ///
    /// Returns a promise for the evaluation of this module and its
    /// dependencies, resolving on successful evaluation or if it has already
    /// been evaluated successfully, and rejecting for an evaluation error or
    /// if it has already been evaluated unsuccessfully. If the promise is
    /// rejected, hosts are expected to handle the promise rejection and
    /// rethrow the evaluation error.
    ///
    /// Link must have completed successfully prior to invoking this method.
    fn evaluate<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> Promise<'gc>;
}

impl AbstractModuleSlots for AbstractModule<'_> {
    fn environment<'a>(
        self,
        agent: &Agent,
        gc: NoGcScope<'a, '_>,
    ) -> Option<ModuleEnvironment<'a>> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.environment(agent, gc),
        }
    }

    fn set_environment(self, agent: &mut Agent, env: ModuleEnvironment) {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.set_environment(agent, env),
        }
    }

    fn namespace<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Option<Module<'a>> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.namespace(agent, gc),
        }
    }

    fn set_namespace(self, agent: &mut Agent, namespace: Module) {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.set_namespace(agent, namespace),
        }
    }

    fn realm<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> Realm<'a> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.realm(agent, gc),
        }
    }

    fn host_defined(self, agent: &Agent) -> Option<HostDefined> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.host_defined(agent),
        }
    }
}

impl AbstractModuleMethods for AbstractModule<'_> {
    fn load_requested_modules<'a>(
        self,
        agent: &mut Agent,
        host_defined: Option<HostDefined>,
        gc: NoGcScope<'a, '_>,
    ) -> Promise<'a> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => {
                m.load_requested_modules(agent, host_defined, gc)
            }
        }
    }

    fn get_exported_names<'a>(
        self,
        agent: &Agent,
        export_start_set: &mut Vec<SourceTextModule<'a>>,
        gc: NoGcScope<'a, '_>,
    ) -> Vec<String<'a>> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => {
                m.get_exported_names(agent, export_start_set, gc)
            }
        }
    }

    fn resolve_export<'a>(
        self,
        agent: &Agent,
        export_name: String,
        resolve_set: &mut Vec<ResolveSetEntry<'a>>,
        gc: NoGcScope<'a, '_>,
    ) -> Option<ResolvedBinding<'a>> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => {
                m.resolve_export(agent, export_name, resolve_set, gc)
            }
        }
    }

    fn link<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> JsResult<'a, ()> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.link(agent, gc),
        }
    }

    fn evaluate<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> Promise<'gc> {
        match self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.evaluate(agent, gc),
        }
    }
}

impl HeapMarkAndSweep for AbstractModule<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match &self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match &mut self.0 {
            InnerAbstractModule::SourceTextModule(m) => m.sweep_values(compactions),
        }
    }
}

impl HeapMarkAndSweep for AbstractModuleRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            realm,
            environment,
            namespace,
            host_defined: _,
        } = self;
        realm.mark_values(queues);
        environment.mark_values(queues);
        namespace.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            realm,
            environment,
            namespace,
            host_defined: _,
        } = self;
        realm.sweep_values(compactions);
        environment.sweep_values(compactions);
        namespace.sweep_values(compactions);
    }
}
