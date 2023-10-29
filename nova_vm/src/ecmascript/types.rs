mod language;
mod spec;

pub use language::{
    BigInt, Function, InternalMethods, Number, Object, OrdinaryObject, PropertyKey, String, Value,
};
pub(crate) use language::{
    BigIntHeapData, FunctionHeapData, NumberHeapData, ObjectHeapData, StringHeapData,
};
pub(crate) use spec::DataBlock;
pub use spec::{Base, PropertyDescriptor, Reference, ReferencedName};

#[derive(Debug)]
pub struct Symbol;
