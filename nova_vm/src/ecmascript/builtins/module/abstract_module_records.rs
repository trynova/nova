use super::Module;
use crate::{
    ecmascript::{
        execution::{ModuleEnvironmentIndex, RealmIdentifier},
        scripts_and_modules::{module::ModuleIdentifier, script::HostDefined},
        types::String,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ResolvedBindingName {
    String(StringIndex),
    SmallString(SmallString),
    Namespace,
}

impl Into<ResolvedBindingName> for String {
    fn into(self) -> ResolvedBindingName {
        match self {
            String::String(d) => ResolvedBindingName::String(d),
            String::SmallString(d) => ResolvedBindingName::SmallString(d),
        }
    }
}

impl ResolvedBindingName {
    pub(crate) fn is_string(&self) -> bool {
        match self {
            ResolvedBindingName::String(_) => true,
            ResolvedBindingName::SmallString(_) => true,
            ResolvedBindingName::Namespace => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResolvedBinding {
    /// \[\[Module]]
    pub(super) module: Option<Module>,
    /// \[\[BindingName]]
    pub(super) binding_name: ResolvedBindingName,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ResolveExportResult {
    Ambiguous,
    Resolved(ResolvedBinding),
}
