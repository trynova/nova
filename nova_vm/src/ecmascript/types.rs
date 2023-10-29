mod language;
mod spec;

pub(crate) use language::{BigIntHeapData, ObjectHeapData, StringHeapData};
pub use language::{Function, InternalMethods, Number, Object, PropertyKey, String, Value};
pub use spec::{Base, PropertyDescriptor, Reference, ReferencedName};

#[derive(Debug)]
pub struct Symbol;
