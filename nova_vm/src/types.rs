mod language;
mod spec;

pub use language::{Function, InternalMethods, Number, Object, PropertyKey, String, Value};
pub use spec::{Base, PropertyDescriptor, Reference, ReferencedName};

#[derive(Debug)]
pub struct Symbol;
