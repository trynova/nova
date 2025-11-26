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

pub(crate) use language::*;
pub use language::{
    BigInt, Function, HeapNumber, HeapString, InternalMethods, InternalSlots, IntoFunction,
    IntoNumeric, IntoObject, IntoPrimitive, IntoValue, NoCache, Number, Numeric, Object,
    OrdinaryObject, Primitive, PropertyKey, PropertyKeySet, String, Symbol, TryGetResult, Value,
    bigint,
};
#[cfg(feature = "shared-array-buffer")]
pub use spec::SharedDataBlock;
pub(crate) use spec::*;
pub use spec::{PrivateName, PropertyDescriptor};
