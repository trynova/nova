// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use std::ops::{Index, IndexMut};

pub use data::SymbolHeapData;

use crate::{
    ecmascript::{execution::Agent, types::String},
    engine::rootable::{HeapRootData, HeapRootRef, Rootable},
    heap::{
        indexes::SymbolIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WellKnownSymbolIndexes, WorkQueues, LAST_WELL_KNOWN_SYMBOL_INDEX,
    },
};

use super::{IntoPrimitive, IntoValue, Primitive, Value, BUILTIN_STRING_MEMORY};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Symbol(pub(crate) SymbolIndex);

/// Inner root repr type to hide WellKnownSymbolIndexes.
#[derive(Debug, Clone, Copy)]
enum SymbolRootReprInner {
    // Note: Handle a special case of avoiding rooting well-known symbols.
    WellKnown(WellKnownSymbolIndexes),
    HeapRef(HeapRootRef),
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct SymbolRootRepr(SymbolRootReprInner);

impl Symbol {
    pub(crate) const fn _def() -> Self {
        Self(SymbolIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

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
            BUILTIN_STRING_MEMORY.Symbol__
        }
    }
}

impl IntoValue for Symbol {
    fn into_value(self) -> Value {
        Value::Symbol(self)
    }
}

impl IntoPrimitive for Symbol {
    fn into_primitive(self) -> Primitive {
        Primitive::Symbol(self)
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

impl TryFrom<Value> for Symbol {
    type Error = ();

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Symbol(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl TryFrom<Primitive> for Symbol {
    type Error = ();

    fn try_from(value: Primitive) -> Result<Self, Self::Error> {
        match value {
            Primitive::Symbol(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl Index<Symbol> for Agent {
    type Output = SymbolHeapData;

    fn index(&self, index: Symbol) -> &Self::Output {
        &self.heap.symbols[index]
    }
}

impl IndexMut<Symbol> for Agent {
    fn index_mut(&mut self, index: Symbol) -> &mut Self::Output {
        &mut self.heap.symbols[index]
    }
}

impl Index<Symbol> for Vec<Option<SymbolHeapData>> {
    type Output = SymbolHeapData;

    fn index(&self, index: Symbol) -> &Self::Output {
        self.get(index.get_index())
            .expect("Symbol out of bounds")
            .as_ref()
            .expect("Symbol slot empty")
    }
}

impl IndexMut<Symbol> for Vec<Option<SymbolHeapData>> {
    fn index_mut(&mut self, index: Symbol) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Symbol out of bounds")
            .as_mut()
            .expect("Symbol slot empty")
    }
}

impl HeapMarkAndSweep for Symbol {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.symbols.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.symbols.shift_index(&mut self.0);
    }
}

impl CreateHeapData<SymbolHeapData, Symbol> for Heap {
    fn create(&mut self, data: SymbolHeapData) -> Symbol {
        self.symbols.push(Some(data));
        Symbol(SymbolIndex::last(&self.symbols))
    }
}

impl Rootable for Symbol {
    type RootRepr = SymbolRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        if value.0.into_u32_index() <= LAST_WELL_KNOWN_SYMBOL_INDEX {
            Ok(SymbolRootRepr(SymbolRootReprInner::WellKnown(
                // SAFETY: Value is within the maximum number of well-known symbol indexes.
                unsafe {
                    std::mem::transmute::<u32, WellKnownSymbolIndexes>(value.0.into_u32_index())
                },
            )))
        } else {
            Err(HeapRootData::Symbol(value))
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match value.0 {
            SymbolRootReprInner::WellKnown(well_known) => Ok(Self(well_known.into())),
            SymbolRootReprInner::HeapRef(heap_root_ref) => Err(heap_root_ref),
        }
    }

    #[inline]
    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        SymbolRootRepr(SymbolRootReprInner::HeapRef(heap_ref))
    }

    #[inline]
    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::Symbol(heap_symbol) => Some(heap_symbol),
            _ => None,
        }
    }
}
