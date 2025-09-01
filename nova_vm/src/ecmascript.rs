// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! [ECMAScript language](https://tc39.es/ecma262/)
//!
//! This module contains everything directly related to the ECMAScript language
//! specification and its implementation in the Nova engine.

pub(crate) mod abstract_operations;
pub(crate) mod builders;
pub mod builtins;
pub mod execution;
pub(crate) use builtins::{fundamental_objects, numbers_and_dates};
pub mod scripts_and_modules;
pub(crate) mod syntax_directed_operations;
pub mod types;
