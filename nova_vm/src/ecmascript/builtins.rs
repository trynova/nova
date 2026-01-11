// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! # [10 Ordinary and Exotic Objects Behaviours](https://tc39.es/ecma262/#sec-ordinary-and-exotic-objects-behaviours)
//!
//! Most things in JavaScript are objects: those are all defined underneath
//! this module.
//!
//! ### Ordinary and exotic objects
//!
//! The ECMAScript specification defines _ordinary objects_ by their _internal
//! methods_: an object which uses the ordinary internal methods is an ordinary
//! object, while objects that override some or all internal methods are
//! _exotic objects_. However, often the more important thing to an object is
//! its list of _internal slots_. Internal slots define what data an object can
//! hold: this data is not necessarily directly readable by JavaScript code but
//! it tends to dictate an object's usage very strongly.
//!
//! Thus according to the ECMAScript specification sense, some objects are
//! ordinary but have extra internal slots compared to plain objects created
//! using JavaScript object literals or other equivalent means. Such objects
//! include ArrayBuffers, Maps, and Sets. From a JavaScript programmer's point
//! of view it is evident that these objects are not "ordinary". Hence, it may
//! be beneficial to think of plain objects as "ordinary" and all other objects
//! as "exotic". This is very much the view that the Nova engine takes: in a
//! very real sense, all non-plain objects have both extra internal slots and
//! override the ordinary internal methods. This is because all such objects
//! in Nova have a special `[[BackingObject]]` slot and their internal methods
//! are modified such that they delegate "ordinary object business" to the
//! backing object if it exists.

pub(crate) mod arguments;
pub(crate) mod array;
#[cfg(feature = "array-buffer")]
pub(crate) mod array_buffer;
pub(crate) mod bound_function;
pub(crate) mod builtin_constructor;
pub(crate) mod builtin_function;
pub(crate) mod control_abstraction_objects;
#[cfg(feature = "array-buffer")]
pub(crate) mod data_view;
#[cfg(feature = "date")]
pub(crate) mod date;
pub(crate) mod ecmascript_function;
pub(crate) mod embedder_object;
pub(crate) mod error;
pub(crate) mod finalization_registry;
pub(crate) mod fundamental_objects;
pub(crate) mod global_object;
pub(crate) mod indexed_collections;
pub(crate) mod keyed_collections;
pub(crate) mod managing_memory;
pub(crate) mod map;
pub(crate) mod module;
pub(crate) mod numbers_and_dates;
pub(crate) mod ordinary;
pub(crate) mod primitive_objects;
pub(crate) mod promise;
pub(crate) mod proxy;
pub(crate) mod reflection;
#[cfg(feature = "regexp")]
pub(crate) mod regexp;
#[cfg(feature = "set")]
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
pub(crate) use array::*;
#[cfg(feature = "array-buffer")]
pub(crate) use array_buffer::*;
pub(crate) use builtin_constructor::*;
pub(crate) use builtin_function::*;
pub(crate) use control_abstraction_objects::*;
pub(crate) use ecmascript_function::*;
#[cfg(feature = "array-buffer")]
pub(crate) use typed_array::*;

pub use array::Array;
#[cfg(feature = "array-buffer")]
pub use array_buffer::ArrayBuffer;
pub use bound_function::BoundFunction;
pub use builtin_function::{
    ArgumentsList, Behaviour, Builtin, BuiltinFunction, BuiltinFunctionArgs, BuiltinGetter,
    BuiltinSetter, ConstructorFn, RegularFn, ScopedArgumentsList, create_builtin_function,
};
pub use ecmascript_function::ECMAScriptFunction;
#[cfg(feature = "shared-array-buffer")]
pub use shared_array_buffer::SharedArrayBuffer;
#[cfg(feature = "array-buffer")]
pub use typed_array::{
    AnyTypedArray, GenericSharedTypedArray, GenericTypedArray, SharedTypedArray, TypedArray,
};
