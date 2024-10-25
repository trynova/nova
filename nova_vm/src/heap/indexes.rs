// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "date")]
use crate::ecmascript::builtins::date::data::DateHeapData;
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::RegExpHeapData;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::builtins::shared_array_buffer::data::SharedArrayBufferHeapData;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::{
    data_view::data::DataViewHeapData, typed_array::data::TypedArrayHeapData, ArrayBufferHeapData,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{
    weak_map::data::WeakMapHeapData, weak_ref::data::WeakRefHeapData,
    weak_set::data::WeakSetHeapData,
};
use crate::ecmascript::{
    builtins::{
        control_abstraction_objects::generator_objects::GeneratorHeapData,
        embedder_object::data::EmbedderObjectHeapData,
        error::ErrorHeapData,
        finalization_registry::data::FinalizationRegistryHeapData,
        indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIteratorHeapData,
        keyed_collections::{
            map_objects::map_iterator_objects::map_iterator::MapIteratorHeapData,
            set_objects::set_iterator_objects::set_iterator::SetIteratorHeapData,
        },
        map::data::MapHeapData,
        primitive_objects::PrimitiveObjectHeapData,
        promise::data::PromiseHeapData,
        proxy::data::ProxyHeapData,
        set::data::SetHeapData,
        ArrayHeapData,
    },
    types::{
        BigIntHeapData, BoundFunctionHeapData, BuiltinConstructorHeapData, BuiltinFunctionHeapData,
        ECMAScriptFunctionHeapData, NumberHeapData, ObjectHeapData, StringHeapData, SymbolHeapData,
        Value,
    },
};
use core::fmt::Debug;
use std::{
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};
use std::{marker::PhantomData, mem::size_of, num::NonZeroU32};

/// A struct containing a non-zero index into an array or
/// vector of `T`s. Due to the non-zero value, the offset
/// in the vector is offset by one.
///
/// This index implies a tracing reference count from this
/// struct to T at the given index.
pub struct BaseIndex<T: ?Sized>(NonZeroU32, PhantomData<T>);

const _INDEX_SIZE_IS_U32: () = assert!(size_of::<BaseIndex<()>>() == size_of::<u32>());
const _OPTION_INDEX_SIZE_IS_U32: () =
    assert!(size_of::<Option<BaseIndex<()>>>() == size_of::<u32>());

impl<T: ?Sized> Debug for BaseIndex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.0.get() != 0);
        (&self.0.get() - 1).fmt(f)
    }
}

impl<T: ?Sized> Clone for BaseIndex<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for BaseIndex<T> {}

impl<T: ?Sized> PartialEq for BaseIndex<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Eq for BaseIndex<T> {}

impl<T: ?Sized> PartialOrd for BaseIndex<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for BaseIndex<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Hash for BaseIndex<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> BaseIndex<T> {
    pub const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }

    pub const fn into_u32_index(self) -> u32 {
        self.0.get() - 1
    }

    pub const fn into_usize(self) -> usize {
        self.0.get() as usize
    }

    pub const fn into_u32(self) -> u32 {
        self.0.get()
    }

    pub const fn from_index(value: usize) -> Self {
        let value = value as u32;
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub const fn from_u32_index(value: u32) -> Self {
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value + 1) }, PhantomData)
    }

    pub const fn from_usize(value: usize) -> Self {
        let value = value as u32;
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value) }, PhantomData)
    }

    pub const fn from_u32(value: u32) -> Self {
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(unsafe { NonZeroU32::new_unchecked(value) }, PhantomData)
    }

    pub fn last(vec: &[Option<T>]) -> Self
    where
        T: Sized,
    {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

impl<T> Default for BaseIndex<T> {
    fn default() -> Self {
        Self::from_u32_index(0)
    }
}

#[cfg(feature = "array-buffer")]
pub type ArrayBufferIndex = BaseIndex<ArrayBufferHeapData>;
pub type ArrayIndex = BaseIndex<ArrayHeapData>;
pub type ArrayIteratorIndex = BaseIndex<ArrayIteratorHeapData>;
pub type BigIntIndex = BaseIndex<BigIntHeapData>;
pub type BoundFunctionIndex = BaseIndex<BoundFunctionHeapData>;
pub type BuiltinFunctionIndex = BaseIndex<BuiltinFunctionHeapData>;
pub type BuiltinConstructorIndex = BaseIndex<BuiltinConstructorHeapData>;
#[cfg(feature = "array-buffer")]
pub type DataViewIndex = BaseIndex<DataViewHeapData>;
#[cfg(feature = "date")]
pub type DateIndex = BaseIndex<DateHeapData>;
pub type ECMAScriptFunctionIndex = BaseIndex<ECMAScriptFunctionHeapData>;
pub type ElementIndex = BaseIndex<[Option<Value>]>;
pub type EmbedderObjectIndex = BaseIndex<EmbedderObjectHeapData>;
pub type ErrorIndex = BaseIndex<ErrorHeapData>;
pub type FinalizationRegistryIndex = BaseIndex<FinalizationRegistryHeapData>;
pub type GeneratorIndex = BaseIndex<GeneratorHeapData>;
pub type MapIndex = BaseIndex<MapHeapData>;
pub type MapIteratorIndex = BaseIndex<MapIteratorHeapData>;
pub type NumberIndex = BaseIndex<NumberHeapData>;
pub type ObjectIndex = BaseIndex<ObjectHeapData>;
pub type PrimitiveObjectIndex = BaseIndex<PrimitiveObjectHeapData>;
pub type PromiseIndex = BaseIndex<PromiseHeapData>;
pub type ProxyIndex = BaseIndex<ProxyHeapData>;
#[cfg(feature = "regexp")]
pub type RegExpIndex = BaseIndex<RegExpHeapData>;
pub type SetIndex = BaseIndex<SetHeapData>;
pub type SetIteratorIndex = BaseIndex<SetIteratorHeapData>;
#[cfg(feature = "shared-array-buffer")]
pub type SharedArrayBufferIndex = BaseIndex<SharedArrayBufferHeapData>;
pub type StringIndex = BaseIndex<StringHeapData>;
pub type SymbolIndex = BaseIndex<SymbolHeapData>;
#[cfg(feature = "array-buffer")]
pub type TypedArrayIndex = BaseIndex<TypedArrayHeapData>;
#[cfg(feature = "weak-refs")]
pub type WeakMapIndex = BaseIndex<WeakMapHeapData>;
#[cfg(feature = "weak-refs")]
pub type WeakRefIndex = BaseIndex<WeakRefHeapData>;
#[cfg(feature = "weak-refs")]
pub type WeakSetIndex = BaseIndex<WeakSetHeapData>;

// Implement Default for ElementIndex: This is done to support Default
// constructor of ElementsVector.
impl Default for ElementIndex {
    fn default() -> Self {
        Self(unsafe { NonZeroU32::new_unchecked(1) }, Default::default())
    }
}

impl ElementIndex {
    pub fn last_element_index<const N: usize>(vec: &[Option<[Option<Value>; N]>]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

impl<const N: usize> Index<ElementIndex> for Vec<Option<[Option<Value>; N]>> {
    type Output = [Option<Value>; N];

    fn index(&self, index: ElementIndex) -> &Self::Output {
        self.get(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
            .as_ref()
            .expect("Invalid ElementsVector: Found None at index")
    }
}

impl<const N: usize> IndexMut<ElementIndex> for Vec<Option<[Option<Value>; N]>> {
    fn index_mut(&mut self, index: ElementIndex) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
            .as_mut()
            .expect("Invalid ElementsVector: Found None at index")
    }
}
