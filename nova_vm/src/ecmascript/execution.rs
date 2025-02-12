// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;

pub use agent::{Agent, JsResult};
pub use default_host_hooks::DefaultHostHooks;
pub(crate) use environments::{
    get_this_environment, new_class_field_initializer_environment,
    new_class_static_element_environment, new_declarative_environment, new_function_environment,
    DeclarativeEnvironmentIndex, EnvironmentIndex, Environments, FunctionEnvironmentIndex,
    GlobalEnvironment, GlobalEnvironmentIndex, ModuleEnvironmentIndex, ObjectEnvironmentIndex,
    PrivateEnvironmentIndex, ThisBindingStatus,
};
pub(crate) use execution_context::*;
#[cfg(test)]
pub(crate) use realm::{create_realm, set_realm_global_object};
pub(crate) use realm::{
    initialize_default_realm, initialize_host_defined_realm, ProtoIntrinsics, Realm,
    RealmIdentifier,
};
