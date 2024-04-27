use super::Module;
use crate::{
    ecmascript::{
        execution::{ModuleEnvironmentIndex, RealmIdentifier},
        scripts_and_modules::{module::ModuleIdentifier, script::HostDefined},
    },
    heap::indexes::StringIndex,
};
use small_string::SmallString;

#[derive(Debug, Clone, Copy)]
pub(crate) struct NotLinkedErr;
#[derive(Debug, Clone, Copy)]
pub(crate) struct NotLoadedErr;

#[derive(Debug)]
pub(crate) struct ModuleRecord {
    /// \[\[Realm]]
    ///
    /// The Realm within which this module was created.
    pub(super) realm: RealmIdentifier,
    /// \[\[Environment]]
    ///
    /// The Environment Record containing the top level bindings for this
    /// module. This field is set when the module is linked.
    pub(super) environment: Option<ModuleEnvironmentIndex>,
    /// \[\[Namespace]]
    ///
    /// The Module Namespace Object (28.3) if one has been created for this
    /// module.
    pub(super) namespace: Option<Module>,
    /// \[\[HostDefined]]
    ///
    /// Field reserved for use by host environments that need to associate
    /// additional information with a module.
    pub(super) host_defined: Option<HostDefined>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolvedBindingName {
    String(StringIndex),
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
