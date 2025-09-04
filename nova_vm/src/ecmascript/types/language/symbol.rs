// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

use core::ops::{Index, IndexMut};

pub use data::SymbolHeapData;

use crate::{
    ecmascript::{execution::Agent, types::String},
    engine::{
        context::{Bindable, NoGcScope},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        LAST_WELL_KNOWN_SYMBOL_INDEX, PropertyKeyHeap, WellKnownSymbolIndexes, WorkQueues,
        indexes::BaseIndex,
    },
};

use super::{BUILTIN_STRING_MEMORY, IntoPrimitive, Primitive, PropertyKey, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Symbol<'a>(BaseIndex<'a, SymbolHeapData<'static>>);

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

impl<'a> Symbol<'a> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    /// Return the name for functions created using NamedEvaluation with a
    /// Symbol property key.
    ///
    /// ### [10.2.9 SetFunctionName ( F, name \[ , prefix \] )](https://tc39.es/ecma262/#sec-setfunctionname)
    /// ```text
    /// 2. If name is a Symbol, then
    /// a. Let description be name's [[Description]] value.
    /// c. Else, set name to the string-concatenation of "[", description, and
    ///    "]".
    /// b. If description is undefined, set name to the empty String.
    /// ```
    pub(crate) fn get_symbol_function_name(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> String<'a> {
        // a. Let description be name's [[Description]] value.
        if let Some(descriptor) = agent[self].descriptor {
            // c. Else, set name to the string-concatenation of
            //    "[", description, and "]".
            let description = descriptor.to_string_lossy(agent);
            String::from_string(agent, format!("[{description}]"), gc)
        } else {
            // b. If description is undefined, set name to the
            //    empty String.
            String::EMPTY_STRING
        }
    }

    /// ### [20.4.3.3.1 SymbolDescriptiveString ( sym )](https://tc39.es/ecma262/#sec-symboldescriptivestring)
    pub fn descriptive_string(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> String<'a> {
        if let Some(descriptor) = agent[self].descriptor {
            String::concat(
                agent,
                [
                    String::from_small_string("Symbol("),
                    descriptor,
                    String::from_small_string(")"),
                ],
                gc,
            )
        } else {
            BUILTIN_STRING_MEMORY.Symbol__
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Symbol<'_> {
    type Of<'a> = Symbol<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl From<WellKnownSymbolIndexes> for Symbol<'static> {
    fn from(value: WellKnownSymbolIndexes) -> Self {
        Symbol(BaseIndex::from_u32_index(value as u32))
    }
}

impl WellKnownSymbolIndexes {
    pub const fn to_property_key(self) -> PropertyKey<'static> {
        PropertyKey::Symbol(Symbol(BaseIndex::from_u32_index(self as u32)))
    }
}

impl<'a> From<Symbol<'a>> for Value<'a> {
    fn from(symbol: Symbol<'a>) -> Self {
        Value::Symbol(symbol.unbind())
    }
}

impl<'a> From<Symbol<'a>> for Primitive<'a> {
    fn from(value: Symbol<'a>) -> Self {
        value.into_primitive()
    }
}

impl<'a> TryFrom<Value<'a>> for Symbol<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Symbol(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Primitive<'a>> for Symbol<'a> {
    type Error = ();

    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        match value {
            Primitive::Symbol(idx) => Ok(idx),
            _ => Err(()),
        }
    }
}

impl Index<Symbol<'_>> for Agent {
    type Output = SymbolHeapData<'static>;

    fn index(&self, index: Symbol<'_>) -> &Self::Output {
        &self.heap.symbols[index]
    }
}

impl IndexMut<Symbol<'_>> for Agent {
    fn index_mut(&mut self, index: Symbol<'_>) -> &mut Self::Output {
        &mut self.heap.symbols[index]
    }
}

impl Index<Symbol<'_>> for PropertyKeyHeap<'_> {
    type Output = SymbolHeapData<'static>;

    fn index(&self, index: Symbol<'_>) -> &Self::Output {
        &self.symbols[index]
    }
}

impl Index<Symbol<'_>> for Vec<Option<SymbolHeapData<'static>>> {
    type Output = SymbolHeapData<'static>;

    fn index(&self, index: Symbol<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Symbol out of bounds")
            .as_ref()
            .expect("Symbol slot empty")
    }
}

impl IndexMut<Symbol<'_>> for Vec<Option<SymbolHeapData<'static>>> {
    fn index_mut(&mut self, index: Symbol<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Symbol out of bounds")
            .as_mut()
            .expect("Symbol slot empty")
    }
}

impl HeapMarkAndSweep for Symbol<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.symbols.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.symbols.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for Symbol<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions.symbols.shift_weak_index(self.0).map(Self)
    }
}

impl<'a> CreateHeapData<SymbolHeapData<'a>, Symbol<'a>> for Heap {
    fn create(&mut self, data: SymbolHeapData<'a>) -> Symbol<'a> {
        self.symbols.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<SymbolHeapData<'static>>>();
        Symbol(BaseIndex::last(&self.symbols))
    }
}

impl Rootable for Symbol<'_> {
    type RootRepr = SymbolRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        if value.0.into_u32_index() <= LAST_WELL_KNOWN_SYMBOL_INDEX {
            Ok(SymbolRootRepr(SymbolRootReprInner::WellKnown(
                // SAFETY: Value is within the maximum number of well-known symbol indexes.
                unsafe {
                    core::mem::transmute::<u32, WellKnownSymbolIndexes>(value.0.into_u32_index())
                },
            )))
        } else {
            Err(HeapRootData::Symbol(value.unbind()))
        }
    }

    #[inline]
    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        match value.0 {
            SymbolRootReprInner::WellKnown(well_known) => Ok(Symbol::from(well_known)),
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
