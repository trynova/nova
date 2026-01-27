// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//!# [ECMAScript language](https://tc39.es/ecma262/)
//!
//! This module is the main entry point into the Nova JavaScript API and its
//! implementation of the ECMAScript language specification.

mod abstract_operations;
mod builders;
mod builtins;
mod execution;
mod scripts_and_modules;
mod syntax_directed_operations;
mod types;

pub(crate) use abstract_operations::*;
pub use builders::*;
pub use builtins::*;
pub use execution::*;
pub use scripts_and_modules::*;
pub(crate) use syntax_directed_operations::*;
pub use types::*;
