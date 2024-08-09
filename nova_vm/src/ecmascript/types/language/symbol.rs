// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use std::ops::{Index, IndexMut};

pub use data::SymbolHeapData;

use crate::{
    ecmascript::{execution::Agent, types::String},
    heap::{
        indexes::SymbolIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, WorkQueues,
    },
};

use super::{IntoPrimitive, IntoValue, Primitive, Value, BUILTIN_STRING_MEMORY};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Symbol<'gen>(pub(crate) SymbolIndex<'gen>);

impl<'gen> Symbol<'gen> {
    pub(crate) const fn _def() -> Self {
        Self(SymbolIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// ### [20.4.3.3.1 SymbolDescriptiveString ( sym )](https://tc39.es/ecma262/#sec-symboldescriptivestring)
    pub fn descriptive_string(self, agent: &mut Agent<'gen>) -> String<'gen> {
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
            BUILTIN_STRING_MEMORY.Symbol__
        }
    }
}

impl<'gen> IntoValue<'gen> for Symbol<'gen> {
    fn into_value(self) -> Value<'gen> {
        Value::Symbol(self)
    }
}

impl<'gen> IntoPrimitive<'gen> for Symbol<'gen> {
    fn into_primitive(self) -> Primitive<'gen> {
        Primitive::Symbol(self)
    }
}

impl<'gen> From<Symbol<'gen>> for Value<'gen> {
    fn from(value: Symbol<'gen>) -> Self {
        value.into_value()
    }
}

impl<'gen> From<Symbol<'gen>> for Primitive<'gen> {
    fn from(value: Symbol<'gen>) -> Self {
        value.into_primitive()
    }
}

impl<'gen> TryFrom<Value<'gen>> for Symbol<'gen> {
    type Error = ();

    fn try_from(value: Value<'gen>) -> Result<Self, Self::Error> {
        match value {
            Value::Symbol(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'gen> TryFrom<Primitive<'gen>> for Symbol<'gen> {
    type Error = ();

    fn try_from(value: Primitive<'gen>) -> Result<Self, Self::Error> {
        match value {
            Primitive::Symbol(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'gen> Index<Symbol<'gen>> for Agent<'gen> {
    type Output = SymbolHeapData<'gen>;

    fn index(&self, index: Symbol<'gen>) -> &Self::Output {
        &self.heap.symbols[index]
    }
}

impl<'gen> IndexMut<Symbol<'gen>> for Agent<'gen> {
    fn index_mut(&mut self, index: Symbol<'gen>) -> &mut Self::Output {
        &mut self.heap.symbols[index]
    }
}

impl<'gen> Index<Symbol<'gen>> for Vec<Option<SymbolHeapData<'gen>>> {
    type Output = SymbolHeapData<'gen>;

    fn index(&self, index: Symbol<'gen>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Symbol out of bounds")
            .as_ref()
            .expect("Symbol slot empty")
    }
}

impl<'gen> IndexMut<Symbol<'gen>> for Vec<Option<SymbolHeapData<'gen>>> {
    fn index_mut(&mut self, index: Symbol<'gen>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Symbol out of bounds")
            .as_mut()
            .expect("Symbol slot empty")
    }
}

impl<'gen> HeapMarkAndSweep<'gen> for Symbol<'gen> {
    fn mark_values(&self, queues: &mut WorkQueues<'gen>) {
        queues.symbols.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.symbols.shift_index(&mut self.0);
    }
}

impl<'gen> CreateHeapData<SymbolHeapData<'gen>, Symbol<'gen>> for Heap<'gen> {
    fn create(&mut self, data: SymbolHeapData<'gen>) -> Symbol<'gen> {
        self.symbols.push(Some(data));
        Symbol(SymbolIndex::last(&self.symbols))
    }
}
