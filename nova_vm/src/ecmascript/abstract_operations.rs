// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # [7 Abstract Operations](https://tc39.es/ecma262/#sec-abstract-operations)
//!
//! These operations are not a part of the ECMAScript language; they are
//! defined here solely to aid the specification of the semantics of the
//! ECMAScript language. Other, more specialized abstract operations are
//! defined throughout this specification.

pub(crate) mod keyed_group;
pub(crate) mod operations_on_iterator_objects;
pub(crate) mod operations_on_objects;
pub(crate) mod testing_and_comparison;
pub(crate) mod type_conversion;
