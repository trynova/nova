mod language;
mod spec;

pub use language::{Number, Object, String, Value};
pub use spec::{Base, Reference, ReferencedName};

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub struct Symbol;
