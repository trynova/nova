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
    DeclarativeEnvironment, Environment, Environments, FunctionEnvironment, GlobalEnvironment,
    GlobalEnvironmentRecord, ModuleEnvironment, ObjectEnvironment, PrivateEnvironment,
    ThisBindingStatus, get_this_environment, new_class_field_initializer_environment,
    new_class_static_element_environment, new_declarative_environment, new_function_environment,
};
pub(crate) use execution_context::*;
pub(crate) use realm::{
    ProtoIntrinsics, Realm, RealmIdentifier, initialize_default_realm,
    initialize_host_defined_realm,
};
