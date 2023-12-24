mod language;
mod spec;

pub use language::{
    bigint, BigInt, Function, InternalMethods, Number, Object, OrdinaryObject,
    OrdinaryObjectInternalSlots, PropertyKey, String, Value,
};
pub(crate) use language::{
    BigIntHeapData, FunctionHeapData, NumberHeapData, ObjectHeapData, StringHeapData,
};
pub(crate) use spec::{Base, DataBlock, Reference};
pub use spec::{PropertyDescriptor, ReferencedName};

#[derive(Debug)]
pub struct Symbol;
