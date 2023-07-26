mod language;
mod spec;

pub use language::{Function, InternalMethods, Number, Object, PropertyKey, String, Value};
pub use spec::{Base, PropertyDescriptor, Reference, ReferencedName};

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub struct Symbol;
