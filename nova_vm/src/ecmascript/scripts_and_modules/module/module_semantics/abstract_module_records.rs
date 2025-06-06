use crate::{
    ecmascript::{
        builtins::{module::Module, promise::Promise},
        execution::{Agent, JsResult, ModuleEnvironment, Realm},
        scripts_and_modules::script::HostDefined,
    },
    engine::context::{Bindable, GcScope, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

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
    pub(super) fn environment(&self) -> ModuleEnvironment<'m> {
        self.environment
            .expect("Attempted to access environment of an unlinked module")
    }

    /// Set \[\[Environment]] to env.
    pub(super) fn set_environment(&mut self, env: ModuleEnvironment) {
        assert!(
            self.environment.replace(env.unbind()).is_none(),
            "Attempted to set module environment twice"
        );
    }

    /// ### \[\[Realm]]
    pub(super) fn realm(&self) -> Realm<'m> {
        self.realm
    }
}

/// ### [Abstract Methods of Module Records](https://tc39.es/ecma262/#table-abstract-methods-of-module-records)
pub trait ModuleAbstractMethods {
    /// ### LoadRequestedModules(\[hostDefined])
    ///
    /// Prepares the module for linking by recursively loading all its
    /// dependencies, and returns a promise.
    fn load_requested_modules<'a>(
        self,
        agent: &mut Agent,
        host_defined: Option<HostDefined>,
        gc: NoGcScope<'a, '_>,
    ) -> Option<Promise<'a>>;

    /// ### GetExportedNames(\[exportStarSet])
    ///
    /// Return a list of all names that are either directly or indirectly
    /// exported from this module.
    ///
    /// LoadRequestedModules must have completed successfully prior to invoking
    /// this method.
    fn get_exported_names(self, agent: &mut Agent, export_start_set: Option<()>, gc: GcScope);

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
    fn resolve_export(self, agent: &mut Agent, resolve_set: Option<()>, gc: GcScope);

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
    fn evalute<'gc>(self, agent: &mut Agent, gc: GcScope<'gc, '_>) -> Option<Promise<'gc>>;
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
