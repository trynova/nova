//! # 10 Ordinary and Exotic Objects Behaviours
//!
//! Currently only contains code related to subsections 10.2, 10.3 and 10.4.
//!
//! https://tc39.es/ecma262/#sec-ordinary-and-exotic-objects-behaviours

mod array;
mod array_buffer;
mod builtin_function;
mod ecmascript_function;
mod number;
pub mod ordinary;

pub(crate) use array::ArrayHeapData;
pub use array::{Array, ArrayConstructor};
pub use array_buffer::ArrayBuffer;
pub(crate) use array_buffer::ArrayBufferHeapData;
pub use builtin_function::{
    create_builtin_function, todo_builtin, ArgumentsList, Behaviour, Builtin, BuiltinFunctionArgs,
    ConstructorFn, RegularFn as JsFunction, RegularFn,
};
pub use ecmascript_function::ECMAScriptFunction;
pub use number::NumberConstructor;
