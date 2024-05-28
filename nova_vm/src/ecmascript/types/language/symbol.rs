mod data;

use std::ops::{Index, IndexMut};

pub use data::SymbolHeapData;

use crate::{
    ecmascript::{execution::Agent, types::String},
    heap::{indexes::SymbolIndex, Heap},
};

use super::{IntoPrimitive, IntoValue, Primitive, Value};

#[derive(Debug, Clone, Copy)]
pub struct Symbol(pub(crate) SymbolIndex);

impl IntoValue for Symbol {
    fn into_value(self) -> Value {
        Value::Symbol(self.0)
    }
}

impl IntoPrimitive for Symbol {
    fn into_primitive(self) -> Primitive {
        Primitive::Symbol(self.0)
    }
}

impl From<Symbol> for Value {
    fn from(value: Symbol) -> Self {
        value.into_value()
    }
}

impl From<Symbol> for Primitive {
    fn from(value: Symbol) -> Self {
        value.into_primitive()
    }
}

impl From<SymbolIndex> for Symbol {
    fn from(value: SymbolIndex) -> Self {
        Self(value)
    }
}

impl From<SymbolIndex> for Primitive {
    fn from(value: SymbolIndex) -> Self {
        Self::Symbol(value)
    }
}

impl From<SymbolIndex> for Value {
    fn from(value: SymbolIndex) -> Self {
        Self::Symbol(value)
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

impl TryFrom<Primitive> for Symbol {
    type Error = ();

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        match value {
            Primitive::Symbol(idx) => Ok(Self(idx)),
            _ => Err(()),
        }
    }
}

impl Index<Symbol> for Agent {
    type Output = SymbolHeapData;

    fn index(&self, index: Symbol) -> &Self::Output {
        &self.heap[index]
    }
}

impl IndexMut<Symbol> for Agent {
    fn index_mut(&mut self, index: Symbol) -> &mut Self::Output {
        &mut self.heap[index]
    }
}

impl Index<Symbol> for Heap {
    type Output = SymbolHeapData;

    fn index(&self, index: Symbol) -> &Self::Output {
        self.symbols
            .get(index.0.into_index())
            .expect("Symbol out of bounds")
            .as_ref()
            .expect("Symbol slot empty")
    }
}

impl IndexMut<Symbol> for Heap {
    fn index_mut(&mut self, index: Symbol) -> &mut Self::Output {
        self.symbols
            .get_mut(index.0.into_index())
            .expect("Symbol out of bounds")
            .as_mut()
            .expect("Symbol slot empty")
    }
}

impl Symbol {
    /// ### [20.4.3.3.1 SymbolDescriptiveString ( sym )](https://tc39.es/ecma262/#sec-symboldescriptivestring)
    pub fn descriptive_string(self, agent: &mut Agent) -> String {
        if let Some(descriptor) = agent[self].descriptor {
            String::concat(
                agent,
                [
                    String::from_small_string("Symbol("),
                    descriptor,
                    String::from_small_string(")"),
                ],
            )
        } else {
            // TODO: Add to builtin_strings
            String::from_static_str(agent, "Symbol()")
        }
    }
}
