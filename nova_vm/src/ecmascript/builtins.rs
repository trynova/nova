// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # 10 Ordinary and Exotic Objects Behaviours
//!
//! Currently only contains code related to subsections 10.2, 10.3 and 10.4.
//!
//! https://tc39.es/ecma262/#sec-ordinary-and-exotic-objects-behaviours

pub(crate) mod arguments;
mod array;
#[cfg(feature = "array-buffer")]
mod array_buffer;
pub mod bound_function;
mod builtin_constructor;
mod builtin_function;
pub(crate) mod control_abstraction_objects;
#[cfg(feature = "array-buffer")]
pub(crate) mod data_view;
#[cfg(feature = "date")]
pub mod date;
mod ecmascript_function;
pub(crate) mod embedder_object;
pub mod error;
pub(crate) mod finalization_registry;
pub(crate) mod fundamental_objects;
pub(crate) mod global_object;
pub(crate) mod indexed_collections;
pub(crate) mod keyed_collections;
pub(crate) mod managing_memory;
pub(crate) mod map;
pub(crate) mod module;
pub(crate) mod numbers_and_dates;
pub mod ordinary;
pub(crate) mod primitive_objects;
pub(crate) mod promise;
pub(crate) mod proxy;
pub(crate) mod reflection;
pub(crate) mod regexp;
pub(crate) mod set;
#[cfg(feature = "shared-array-buffer")]
pub(crate) mod shared_array_buffer;
pub(crate) mod structured_data;
pub(crate) mod text_processing;
#[cfg(feature = "array-buffer")]
pub(crate) mod typed_array;
#[cfg(feature = "weak-refs")]
pub(crate) mod weak_map;
#[cfg(feature = "weak-refs")]
pub(crate) mod weak_ref;
#[cfg(feature = "weak-refs")]
pub(crate) mod weak_set;

pub(crate) use arguments::*;
pub(crate) use array::abstract_operations::*;
pub use array::Array;
pub(crate) use array::{ArrayHeapData, SealableElementsVector};
#[cfg(feature = "array-buffer")]
pub use array_buffer::ArrayBuffer;
#[cfg(feature = "array-buffer")]
pub(crate) use array_buffer::ArrayBufferHeapData;
pub use builtin_constructor::BuiltinConstructorFunction;
pub(crate) use builtin_constructor::{create_builtin_constructor, BuiltinConstructorArgs};
pub use builtin_function::{
    create_builtin_function, ArgumentsList, Behaviour, Builtin, BuiltinFunction,
    BuiltinFunctionArgs, BuiltinGetter, ConstructorFn, RegularFn as JsFunction, RegularFn,
};
pub(crate) use builtin_function::{BuiltinIntrinsic, BuiltinIntrinsicConstructor};
pub use control_abstraction_objects::*;
pub(crate) use ecmascript_function::*;
