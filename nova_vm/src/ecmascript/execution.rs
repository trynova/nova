// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//!# [9 Executable Code and Execution Contexts](https://tc39.es/ecma262/#sec-executable-code-and-execution-contexts)

pub mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;
#[cfg(feature = "weak-refs")]
mod weak_key;
#[cfg(feature = "weak-refs")]
mod weak_ref_and_finalization_registry;

pub use agent::{Agent, JsResult};
pub use default_host_hooks::DefaultHostHooks;
pub(crate) use environments::*;
pub(crate) use execution_context::*;
pub(crate) use realm::{
    ProtoIntrinsics, Realm, RealmRecord, initialize_default_realm, initialize_host_defined_realm,
};
#[cfg(feature = "weak-refs")]
pub(crate) use weak_key::*;
#[cfg(feature = "weak-refs")]
pub(crate) use weak_ref_and_finalization_registry::*;
