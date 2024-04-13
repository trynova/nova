//! # 10 Ordinary and Exotic Objects Behaviours
//!
//! Currently only contains code related to subsections 10.2, 10.3 and 10.4.
//!
//! https://tc39.es/ecma262/#sec-ordinary-and-exotic-objects-behaviours

pub(crate) mod arguments;
mod array;
mod array_buffer;
mod builtin_function;
pub mod date;
mod ecmascript_function;
pub mod error;
pub(crate) mod fundamental_objects;
pub(crate) mod numbers_and_dates;
pub mod ordinary;

pub(crate) use arguments::*;
pub(crate) use array::abstract_operations::*;
pub use array::{Array, ArrayConstructor};
pub(crate) use array::{ArrayHeapData, SealableElementsVector};
pub use array_buffer::ArrayBuffer;
pub(crate) use array_buffer::ArrayBufferHeapData;
pub use builtin_function::{
    create_builtin_function, todo_builtin, ArgumentsList, Behaviour, Builtin, BuiltinFunction,
    BuiltinFunctionArgs, ConstructorFn, RegularFn as JsFunction, RegularFn,
};
pub(crate) use ecmascript_function::*;
