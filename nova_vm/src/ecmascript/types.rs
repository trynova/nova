mod language;
mod spec;

pub use language::{
    bigint, BigInt, Function, InternalMethods, IntoFunction, IntoNumeric, IntoObject,
    IntoPrimitive, IntoValue, Number, Numeric, Object, OrdinaryObject, OrdinaryObjectInternalSlots,
    Primitive, PropertyKey, String, Symbol, Value,
};
pub(crate) use language::{
    BigIntHeapData, BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
    NumberHeapData, ObjectHeapData, StringHeapData, SymbolHeapData, BUILTIN_STRINGS_LIST,
    BUILTIN_STRING_MEMORY,
};
pub(crate) use spec::*;
pub use spec::{PropertyDescriptor, ReferencedName};
