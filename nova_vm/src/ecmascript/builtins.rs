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

mod arguments;
mod array;
#[cfg(feature = "array-buffer")]
mod array_buffer;
mod bound_function;
mod builtin_constructor;
mod builtin_function;
mod control_abstraction_objects;
#[cfg(feature = "array-buffer")]
mod data_view;
#[cfg(feature = "date")]
mod date;
mod ecmascript_function;
mod embedder_object;
mod error;
mod finalization_registry;
mod fundamental_objects;
mod global_object;
mod indexed_collections;
mod keyed_collections;
mod managing_memory;
mod map;
mod module;
mod numbers_and_dates;
mod ordinary;
mod primitive_objects;
mod promise;
mod proxy;
mod reflection;
#[cfg(feature = "regexp")]
mod regexp;
#[cfg(feature = "set")]
mod set;
#[cfg(feature = "shared-array-buffer")]
mod shared_array_buffer;
mod structured_data;
#[cfg(feature = "temporal")]
mod temporal;
mod text_processing;
#[cfg(feature = "array-buffer")]
mod typed_array;
#[cfg(feature = "weak-refs")]
mod weak_map;
#[cfg(feature = "weak-refs")]
mod weak_ref;
#[cfg(feature = "weak-refs")]
mod weak_set;

pub(crate) use arguments::*;
pub use array::*;
#[cfg(feature = "array-buffer")]
pub use array_buffer::*;
pub use bound_function::*;
pub use builtin_constructor::*;
pub use builtin_function::*;
pub use control_abstraction_objects::*;
#[cfg(feature = "array-buffer")]
pub use data_view::*;
#[cfg(feature = "date")]
pub use date::*;
pub use ecmascript_function::*;
pub use embedder_object::*;
pub use error::*;
pub use finalization_registry::*;
pub(crate) use fundamental_objects::*;
pub(crate) use global_object::*;
pub use indexed_collections::*;
pub use keyed_collections::*;
pub(crate) use managing_memory::*;
pub use map::*;
pub use module::*;
pub(crate) use numbers_and_dates::*;
pub use ordinary::*;
pub use primitive_objects::*;
pub use promise::*;
pub use proxy::*;
pub(crate) use reflection::*;
#[cfg(feature = "regexp")]
pub use regexp::*;
#[cfg(feature = "set")]
pub use set::*;
#[cfg(feature = "shared-array-buffer")]
pub use shared_array_buffer::*;
pub(crate) use structured_data::*;
#[cfg(feature = "temporal")]
pub use temporal::*;
pub use text_processing::*;
#[cfg(feature = "array-buffer")]
pub use typed_array::*;
#[cfg(feature = "weak-refs")]
pub use weak_map::*;
#[cfg(feature = "weak-refs")]
pub use weak_ref::*;
#[cfg(feature = "weak-refs")]
pub use weak_set::*;
