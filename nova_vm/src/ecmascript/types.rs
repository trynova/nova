mod language;
mod spec;

pub use language::{
    bigint, BigInt, Function, InternalMethods, IntoFunction, IntoObject, IntoValue, Number, Object,
    OrdinaryObject, OrdinaryObjectInternalSlots, PropertyKey, String, Symbol, Value,
};
pub(crate) use language::{
    BigIntHeapData, BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
    NumberHeapData, ObjectHeapData, StringHeapData, SymbolHeapData,
};
pub(crate) use spec::*;
pub use spec::{PropertyDescriptor, ReferencedName};
