pub mod bigint;
mod function;
mod number;
mod object;
mod string;
mod value;

pub use bigint::{BigInt, BigIntHeapData};
pub use function::Function;
pub(crate) use function::{
    BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
};
pub use number::{Number, NumberHeapData};
pub use object::{
    InternalMethods, Object, ObjectHeapData, OrdinaryObject, OrdinaryObjectInternalSlots,
    PropertyKey,
};
pub use string::{String, StringHeapData};
pub use value::Value;
