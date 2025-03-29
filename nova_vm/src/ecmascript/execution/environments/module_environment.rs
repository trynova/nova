// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::DeclarativeEnvironment;

/// ### [9.1.1.5 Module Environment Records](https://tc39.es/ecma262/#sec-module-environment-records)
/// A Module Environment Record is a Declarative Environment Record that is
/// used to represent the outer scope of an ECMAScript Module. In additional to
/// normal mutable and immutable bindings, Module Environment Records also
/// provide immutable import bindings which are bindings that provide indirect
/// access to a target binding that exists in another Environment Record.
///
/// Module Environment Records support all of the Declarative Environment
/// Record methods listed in Table 16 and share the same specifications for all
/// of those methods except for GetBindingValue, DeleteBinding, HasThisBinding
/// and GetThisBinding.
///
/// NOTE: There is no data-wise difference between a DeclarativeEnvironment and
/// a ModuleEnvironment, so we treat them exactly the same way.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ModuleEnvironment(DeclarativeEnvironment);
