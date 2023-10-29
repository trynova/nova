mod language;
mod spec;

pub(crate) use language::{
    BigIntHeapData, FunctionHeapData, NumberHeapData, ObjectHeapData, StringHeapData,
};
pub use language::{
    Function, InternalMethods, Number, Object, OrdinaryObject, PropertyKey, String, Value,
};
pub(crate) use spec::DataBlock;
pub use spec::{Base, PropertyDescriptor, Reference, ReferencedName};

#[derive(Debug)]
pub struct Symbol;
