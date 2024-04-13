mod data;

pub use data::SymbolHeapData;

use crate::heap::indexes::SymbolIndex;

use super::{IntoValue, Value};

#[derive(Debug, Clone, Copy)]
pub struct Symbol(pub(crate) SymbolIndex);

impl IntoValue for Symbol {
    fn into_value(self) -> Value {
        Value::Symbol(self.0)
    }
}

impl From<Symbol> for Value {
    fn from(value: Symbol) -> Self {
        value.into_value()
    }
}

impl From<SymbolIndex> for Symbol {
    fn from(value: SymbolIndex) -> Self {
        Symbol(value)
    }
}

impl TryFrom<Value> for Symbol {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Symbol(idx) => Ok(Self(idx)),
            _ => Err(()),
        }
    }
}
