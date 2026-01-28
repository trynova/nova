// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//!# [9 Executable Code and Execution Contexts](https://tc39.es/ecma262/#sec-executable-code-and-execution-contexts)

mod agent;
mod default_host_hooks;
mod environments;
mod execution_context;
mod realm;
#[cfg(feature = "weak-refs")]
mod weak_key;
#[cfg(feature = "weak-refs")]
mod weak_ref_and_finalization_registry;

pub use agent::*;
pub use default_host_hooks::*;
pub use environments::*;
pub(crate) use execution_context::*;
pub use realm::*;
#[cfg(feature = "weak-refs")]
pub(crate) use weak_key::*;
#[cfg(feature = "weak-refs")]
pub(crate) use weak_ref_and_finalization_registry::*;
