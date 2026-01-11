// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod data;

pub use data::SymbolHeapData;

use crate::{
    ecmascript::{
        execution::Agent,
        types::{Primitive, String, Value},
    },
    engine::{
        context::{Bindable, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        ArenaAccess, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        HeapSweepWeakReference, WellKnownSymbolIndexes, WorkQueues, arena_vec_access,
        indexes::{BaseIndex, HeapIndexHandle},
    },
};

use super::{BUILTIN_STRING_MEMORY, PropertyKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Symbol<'a>(BaseIndex<'a, SymbolHeapData<'static>>);
bindable_handle!(Symbol);
arena_vec_access!(
    Symbol,
    'a,
    SymbolHeapData,
    symbols
);

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
        if let Some(descriptor) = self.get(agent).descriptor {
            // c. Else, set name to the string-concatenation of
            //    "[", description, and "]".
            let description = descriptor.to_string_lossy_(agent);
            String::from_string(agent, format!("[{description}]"), gc)
        } else {
            // b. If description is undefined, set name to the empty String.
            String::EMPTY_STRING.bind(gc)
        }
    }

    /// ### [20.4.3.3.1 SymbolDescriptiveString ( sym )](https://tc39.es/ecma262/#sec-symboldescriptivestring)
    pub fn descriptive_string(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> String<'a> {
        if let Some(descriptor) = self.get(agent).descriptor {
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

impl From<WellKnownSymbolIndexes> for Symbol<'static> {
    fn from(value: WellKnownSymbolIndexes) -> Self {
        Symbol(BaseIndex::from_index_u32(value as u32))
    }
}

impl WellKnownSymbolIndexes {
    pub const fn to_property_key(self) -> PropertyKey<'static> {
        PropertyKey::Symbol(Symbol(BaseIndex::from_index_const(self as u32 as usize)))
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
        self.symbols.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<SymbolHeapData<'static>>();
        Symbol(BaseIndex::last(&self.symbols))
    }
}

// === OUTPUT OF primitive_handle! MACRO ADAPTED FOR Symbol ===
impl HeapIndexHandle for Symbol<'_> {
    const _DEF: Self = Self(BaseIndex::MAX);
    #[inline]
    fn from_index_u32(index: u32) -> Self {
        Self(BaseIndex::from_index_u32(index))
    }
    #[inline]
    fn get_index_u32(self) -> u32 {
        self.0.get_index_u32()
    }
}
impl Rootable for Symbol<'_> {
    type RootRepr = SymbolRootRepr;

    #[inline]
    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        WellKnownSymbolIndexes::try_from(value)
            .map(|s| SymbolRootRepr(SymbolRootReprInner::WellKnown(s)))
            .map_err(|_| HeapRootData::Symbol(value.unbind()))
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
            HeapRootData::Symbol(s) => Some(s),
            _ => None,
        }
    }
}
impl TryFrom<HeapRootData> for Symbol<'_> {
    type Error = ();
    #[inline]
    fn try_from(value: HeapRootData) -> Result<Self, Self::Error> {
        match value {
            HeapRootData::Symbol(data) => Ok(data),
            _ => Err(()),
        }
    }
}
impl<'a> From<Symbol<'a>> for Value<'a> {
    #[inline(always)]
    fn from(value: Symbol<'a>) -> Self {
        Self::Symbol(value)
    }
}
impl<'a> TryFrom<Value<'a>> for Symbol<'a> {
    type Error = ();
    #[inline]
    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::Symbol(data) => Ok(data),
            _ => Err(()),
        }
    }
}
impl<'a> From<Symbol<'a>> for Primitive<'a> {
    #[inline(always)]
    fn from(value: Symbol<'a>) -> Self {
        Self::Symbol(value)
    }
}
impl<'a> TryFrom<Primitive<'a>> for Symbol<'a> {
    type Error = ();
    #[inline]
    fn try_from(value: Primitive<'a>) -> Result<Self, Self::Error> {
        match value {
            Primitive::Symbol(data) => Ok(data),
            _ => Err(()),
        }
    }
}
// === END ===
