mod language;
mod spec;

pub use language::Value;
pub use spec::{Base, Reference, ReferencedName};

impl From<Object> for Value {
    fn from(value: Object) -> Self {
        todo!()
    }
}

#[derive(Debug)]
pub struct Object;

#[derive(Debug)]
pub struct Symbol;
