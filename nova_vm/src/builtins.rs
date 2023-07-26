mod array;
mod builtin_function;
mod ecmascript_function;
mod number;
pub mod ordinary;

pub use array::ArrayConstructor;
pub use builtin_function::{
    create_builtin_function, todo_builtin, ArgumentsList, Behaviour, Builtin, BuiltinFunctionArgs,
    ConstructorFn, RegularFn as JsFunction, RegularFn,
};
pub use ecmascript_function::ECMAScriptFunction;
pub use number::NumberConstructor;
