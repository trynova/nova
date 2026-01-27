// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # [6 ECMAScript Data Types and Values](https://tc39.es/ecma262/#sec-ecmascript-data-types-and-values)
//!
//! This module groups together the ECMAScript Data Types and Values section,
//! the Ordinary and Exotic Object Behaviours section, all the "ECMAScript
//! Language" sections (11-16), and the builtin object sections (18-29).

mod language;
mod spec;

pub use language::*;
pub use spec::*;
