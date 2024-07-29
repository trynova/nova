// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::{
    builtins::{
        control_abstraction_objects::generator_objects::GeneratorHeapData,
        data_view::data::DataViewHeapData, date::data::DateHeapData,
        embedder_object::data::EmbedderObjectHeapData, error::ErrorHeapData,
        finalization_registry::data::FinalizationRegistryHeapData, map::data::MapHeapData,
        primitive_objects::PrimitiveObjectHeapData, promise::data::PromiseHeapData,
        proxy::data::ProxyHeapData, regexp::RegExpHeapData, set::data::SetHeapData,
        shared_array_buffer::data::SharedArrayBufferHeapData,
        typed_array::data::TypedArrayHeapData, weak_map::data::WeakMapHeapData,
        weak_ref::data::WeakRefHeapData, weak_set::data::WeakSetHeapData, ArrayBufferHeapData,
        ArrayHeapData,
    },
    types::{
        BigIntHeapData, BoundFunctionHeapData, BuiltinFunctionHeapData, ECMAScriptFunctionHeapData,
        NumberHeapData, ObjectHeapData, StringHeapData, SymbolHeapData, Value,
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
/// This index implies a tracing garbage collection from this
/// struct to a T at the given index.
/// 
/// ### Lifetime
/// 
/// The index contains a `'gen` lifetime. This is the "generation" of a
/// JavaScript value. An old generation's index may no longer point to the same
/// value after garbage collection.
#[repr(transparent)]
pub struct BaseIndex<'gen, T: ?Sized>(NonZeroU32, PhantomData<&'gen T>);

const _INDEX_SIZE_IS_U32: () = assert!(size_of::<BaseIndex<()>>() == size_of::<u32>());
const _OPTION_INDEX_SIZE_IS_U32: () =
    assert!(size_of::<Option<BaseIndex<()>>>() == size_of::<u32>());

impl<'gen, T: ?Sized> Debug for BaseIndex<'gen, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.0.get() != 0);
        (&self.0.get() - 1).fmt(f)
    }
}

impl<'gen, T: ?Sized> Clone for BaseIndex<'gen, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'gen, T: ?Sized> Copy for BaseIndex<'gen, T> {}

impl<'gen, T: ?Sized> PartialEq for BaseIndex<'gen, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<'gen, T: ?Sized> Eq for BaseIndex<'gen, T> {}

impl<'gen, T: ?Sized> PartialOrd for BaseIndex<'gen, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'gen, T: ?Sized> Ord for BaseIndex<'gen, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<'gen, T: ?Sized> Hash for BaseIndex<'gen, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> BaseIndex<'_, T> {
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
}

impl<'gen, T: ?Sized> BaseIndex<'gen, T> {
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

impl<'gen, T> Default for BaseIndex<'gen, T> {
    fn default() -> Self {
        Self::from_u32_index(0)
    }
}

pub type ArrayBufferIndex<'gen> = BaseIndex<'gen, ArrayBufferHeapData<'gen>>;
pub type ArrayIndex<'gen> = BaseIndex<'gen, ArrayHeapData<'gen>>;
pub type BigIntIndex<'gen> = BaseIndex<'gen, BigIntHeapData>;
pub type BoundFunctionIndex<'gen> = BaseIndex<'gen, BoundFunctionHeapData<'gen>>;
pub type BuiltinFunctionIndex<'gen> = BaseIndex<'gen, BuiltinFunctionHeapData<'gen>>;
pub type DataViewIndex<'gen> = BaseIndex<'gen, DataViewHeapData<'gen>>;
pub type DateIndex<'gen> = BaseIndex<'gen, DateHeapData<'gen>>;
pub type ECMAScriptFunctionIndex<'gen> = BaseIndex<'gen, ECMAScriptFunctionHeapData<'gen>>;
pub type ElementIndex<'gen> = BaseIndex<'gen, [Option<Value<'gen>>]>;
pub type EmbedderObjectIndex<'gen> = BaseIndex<'gen, EmbedderObjectHeapData>;
pub type ErrorIndex<'gen> = BaseIndex<'gen, ErrorHeapData<'gen>>;
pub type FinalizationRegistryIndex<'gen> = BaseIndex<'gen, FinalizationRegistryHeapData<'gen>>;
pub type GeneratorIndex<'gen> = BaseIndex<'gen, GeneratorHeapData<'gen>>;
pub type MapIndex<'gen> = BaseIndex<'gen, MapHeapData<'gen>>;
pub type NumberIndex<'gen> = BaseIndex<'gen, NumberHeapData<'gen>>;
pub type ObjectIndex<'gen> = BaseIndex<'gen, ObjectHeapData<'gen>>;
pub type PrimitiveObjectIndex<'gen> = BaseIndex<'gen, PrimitiveObjectHeapData<'gen>>;
pub type PromiseIndex<'gen> = BaseIndex<'gen, PromiseHeapData<'gen>>;
pub type ProxyIndex<'gen> = BaseIndex<'gen, ProxyHeapData<'gen>>;
pub type RegExpIndex<'gen> = BaseIndex<'gen, RegExpHeapData<'gen>>;
pub type SetIndex<'gen> = BaseIndex<'gen, SetHeapData<'gen>>;
pub type SharedArrayBufferIndex<'gen> = BaseIndex<'gen, SharedArrayBufferHeapData<'gen>>;
pub type StringIndex<'gen> = BaseIndex<'gen, StringHeapData<'gen>>;
pub type SymbolIndex<'gen> = BaseIndex<'gen, SymbolHeapData<'gen>>;
pub type TypedArrayIndex<'gen> = BaseIndex<'gen, TypedArrayHeapData<'gen>>;
pub type WeakMapIndex<'gen> = BaseIndex<'gen, WeakMapHeapData<'gen>>;
pub type WeakRefIndex<'gen> = BaseIndex<'gen, WeakRefHeapData<'gen>>;
pub type WeakSetIndex<'gen> = BaseIndex<'gen, WeakSetHeapData<'gen>>;

// Implement Default for ElementIndex: This is done to support Default
// constructor of ElementsVector.
impl Default for ElementIndex<'_> {
    fn default() -> Self {
        Self(unsafe { NonZeroU32::new_unchecked(1) }, Default::default())
    }
}

impl<'gen> ElementIndex<'gen> {
    pub fn last_element_index<const N: usize>(vec: &[Option<[Option<Value<'gen>>; N]>]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

impl<'gen, const N: usize> Index<ElementIndex<'gen>> for Vec<Option<[Option<Value<'gen>>; N]>> {
    type Output = [Option<Value<'gen>>; N];

    fn index(&self, index: ElementIndex<'gen>) -> &Self::Output {
        self.get(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
            .as_ref()
            .expect("Invalid ElementsVector: Found None at index")
    }
}

impl<'gen, const N: usize> IndexMut<ElementIndex<'gen>> for Vec<Option<[Option<Value<'gen>>; N]>> {
    fn index_mut(&mut self, index: ElementIndex<'gen>) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
            .as_mut()
            .expect("Invalid ElementsVector: Found None at index")
    }
}
