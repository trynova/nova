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

pub(crate) use builtin_constructor::BuiltinConstructorArgs;
pub(crate) use builtin_function::{BuiltinIntrinsic, BuiltinIntrinsicConstructor};
pub(crate) use ecmascript_function::{
    ConstructorStatus, FunctionAstRef, OrdinaryFunctionCreateParams, SetFunctionNamePrefix,
};
#[cfg(feature = "shared-array-buffer")]
pub(crate) use typed_array::SharedVoidArray;
#[cfg(feature = "array-buffer")]
pub(crate) use typed_array::VoidArray;

pub use array::Array;
#[cfg(feature = "array-buffer")]
pub use array_buffer::{AnyArrayBuffer, ArrayBuffer};
pub use bound_function::BoundFunction;
pub use builtin_constructor::BuiltinConstructorFunction;
pub use builtin_function::{
    ArgumentsList, Behaviour, Builtin, BuiltinFunction, BuiltinFunctionArgs, BuiltinGetter,
    BuiltinSetter, ConstructorFn, RegularFn, ScopedArgumentsList, create_builtin_function,
};
pub use control_abstraction_objects::{
    async_generator_objects::AsyncGenerator,
    generator_objects::Generator,
    promise_objects::promise_abstract_operations::{
        promise_capability_records::PromiseCapability,
        promise_finally_functions::BuiltinPromiseFinallyFunction,
        promise_resolving_functions::BuiltinPromiseResolvingFunction,
    },
};
#[cfg(feature = "shared-array-buffer")]
pub use data_view::SharedDataView;
#[cfg(feature = "array-buffer")]
pub use data_view::{AnyDataView, DataView};
#[cfg(feature = "date")]
pub use date::Date;
pub use ecmascript_function::ECMAScriptFunction;
pub use embedder_object::EmbedderObject;
pub use error::Error;
pub use finalization_registry::FinalizationRegistry;
pub use indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIterator;
pub use keyed_collections::{
    map_objects::map_iterator_objects::map_iterator::MapIterator,
    set_objects::set_iterator_objects::set_iterator::SetIterator,
};
pub use map::Map;
pub use module::Module;
pub use primitive_objects::PrimitiveObject;
pub use promise::Promise;
pub use proxy::Proxy;
#[cfg(feature = "regexp")]
pub use regexp::RegExp;
#[cfg(feature = "set")]
pub use set::Set;
#[cfg(feature = "shared-array-buffer")]
pub use shared_array_buffer::SharedArrayBuffer;
pub use text_processing::{
    regexp_objects::regexp_string_iterator_objects::RegExpStringIterator,
    string_objects::string_iterator_objects::StringIterator,
};
#[cfg(feature = "array-buffer")]
pub use typed_array::{
    AnyTypedArray, BigInt64Array, BigUint64Array, Float32Array, Float64Array, GenericTypedArray,
    Int8Array, Int16Array, Int32Array, TypedArray, Uint8Array, Uint8ClampedArray, Uint16Array,
    Uint32Array,
};
#[cfg(feature = "shared-array-buffer")]
pub use typed_array::{
    GenericSharedTypedArray, SharedBigInt64Array, SharedBigUint64Array, SharedFloat32Array,
    SharedFloat64Array, SharedInt8Array, SharedInt16Array, SharedInt32Array, SharedTypedArray,
    SharedUint8Array, SharedUint8ClampedArray, SharedUint16Array, SharedUint32Array,
};
#[cfg(feature = "weak-refs")]
pub use weak_map::WeakMap;
#[cfg(feature = "weak-refs")]
pub use weak_ref::WeakRef;
#[cfg(feature = "weak-refs")]
pub use weak_set::WeakSet;
