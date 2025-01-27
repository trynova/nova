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
#[cfg(feature = "set")]
use crate::ecmascript::builtins::{
    keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIteratorHeapData,
    set::data::SetHeapData,
};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::builtins::{
    weak_map::data::WeakMapHeapData, weak_ref::data::WeakRefHeapData,
    weak_set::data::WeakSetHeapData,
};
use crate::{
    ecmascript::{
        builtins::{
            control_abstraction_objects::generator_objects::GeneratorHeapData,
            embedder_object::data::EmbedderObjectHeapData, error::ErrorHeapData,
            finalization_registry::data::FinalizationRegistryHeapData,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIteratorHeapData,
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIteratorHeapData,
            map::data::MapHeapData, primitive_objects::PrimitiveObjectHeapData,
            promise::data::PromiseHeapData, proxy::data::ProxyHeapData, ArrayHeapData,
        },
        types::{
            BigIntHeapData, BoundFunctionHeapData, BuiltinConstructorHeapData,
            BuiltinFunctionHeapData, ECMAScriptFunctionHeapData, NumberHeapData, ObjectHeapData,
            StringHeapData, SymbolHeapData, Value,
        },
    },
    engine::context::{GcToken, NoGcScope},
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
pub struct BaseIndex<'a, T: ?Sized>(NonZeroU32, PhantomData<T>, PhantomData<&'a GcToken>);

const _INDEX_SIZE_IS_U32: () = assert!(size_of::<BaseIndex<()>>() == size_of::<u32>());
const _OPTION_INDEX_SIZE_IS_U32: () =
    assert!(size_of::<Option<BaseIndex<()>>>() == size_of::<u32>());

pub(crate) trait IntoBaseIndex<'a, T: ?Sized> {
    fn into_base_index(self) -> BaseIndex<'a, T>;
}

impl<'a, T: ?Sized> Debug for BaseIndex<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        assert!(self.0.get() != 0);
        (&self.0.get() - 1).fmt(f)
    }
}

impl<'a, T: ?Sized> Clone for BaseIndex<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: ?Sized> Copy for BaseIndex<'a, T> {}

impl<'a, T: ?Sized> PartialEq for BaseIndex<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<'a, T: ?Sized> Eq for BaseIndex<'a, T> {}

impl<'a, T: ?Sized> PartialOrd for BaseIndex<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, T: ?Sized> Ord for BaseIndex<'a, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<'a, T: ?Sized> Hash for BaseIndex<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<'a, T: ?Sized> BaseIndex<'a, T> {
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
        Self(
            unsafe { NonZeroU32::new_unchecked(value + 1) },
            PhantomData,
            PhantomData,
        )
    }

    pub const fn from_u32_index(value: u32) -> Self {
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(
            unsafe { NonZeroU32::new_unchecked(value + 1) },
            PhantomData,
            PhantomData,
        )
    }

    pub const fn from_usize(value: usize) -> Self {
        let value = value as u32;
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(
            unsafe { NonZeroU32::new_unchecked(value) },
            PhantomData,
            PhantomData,
        )
    }

    pub const fn from_u32(value: u32) -> Self {
        assert!(value != 0);
        // SAFETY: Number is not zero.
        // This check is done manually to allow const context.
        Self(
            unsafe { NonZeroU32::new_unchecked(value) },
            PhantomData,
            PhantomData,
        )
    }

    pub fn last(vec: &[Option<T>]) -> Self
    where
        T: Sized,
    {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

impl<'a, T> Default for BaseIndex<'a, T> {
    fn default() -> Self {
        Self::from_u32_index(0)
    }
}

#[cfg(feature = "array-buffer")]
pub type ArrayBufferIndex<'a> = BaseIndex<'a, ArrayBufferHeapData>;
pub type ArrayIndex<'a> = BaseIndex<'a, ArrayHeapData>;
pub type ArrayIteratorIndex<'a> = BaseIndex<'a, ArrayIteratorHeapData>;
pub type BigIntIndex<'a> = BaseIndex<'a, BigIntHeapData>;
pub type BoundFunctionIndex<'a> = BaseIndex<'a, BoundFunctionHeapData>;
pub type BuiltinFunctionIndex<'a> = BaseIndex<'a, BuiltinFunctionHeapData>;
pub type BuiltinConstructorIndex<'a> = BaseIndex<'a, BuiltinConstructorHeapData>;
#[cfg(feature = "array-buffer")]
pub type DataViewIndex<'a> = BaseIndex<'a, DataViewHeapData>;
#[cfg(feature = "date")]
pub type DateIndex<'a> = BaseIndex<'a, DateHeapData>;
pub type ECMAScriptFunctionIndex<'a> = BaseIndex<'a, ECMAScriptFunctionHeapData>;
pub type ElementIndex = BaseIndex<'static, [Option<Value>]>;
pub type EmbedderObjectIndex<'a> = BaseIndex<'a, EmbedderObjectHeapData>;
pub type ErrorIndex<'a> = BaseIndex<'a, ErrorHeapData>;
pub type FinalizationRegistryIndex<'a> = BaseIndex<'a, FinalizationRegistryHeapData>;
pub type GeneratorIndex<'a> = BaseIndex<'a, GeneratorHeapData>;
pub type MapIndex<'a> = BaseIndex<'a, MapHeapData>;
pub type MapIteratorIndex<'a> = BaseIndex<'a, MapIteratorHeapData>;
pub type NumberIndex<'a> = BaseIndex<'a, NumberHeapData>;
pub type ObjectIndex<'a> = BaseIndex<'a, ObjectHeapData>;
pub type PrimitiveObjectIndex<'a> = BaseIndex<'a, PrimitiveObjectHeapData>;
pub type PromiseIndex<'a> = BaseIndex<'a, PromiseHeapData>;
pub type ProxyIndex<'a> = BaseIndex<'a, ProxyHeapData>;
#[cfg(feature = "regexp")]
pub type RegExpIndex<'a> = BaseIndex<'a, RegExpHeapData>;
#[cfg(feature = "set")]
pub type SetIndex<'a> = BaseIndex<'a, SetHeapData>;
#[cfg(feature = "set")]
pub type SetIteratorIndex<'a> = BaseIndex<'a, SetIteratorHeapData>;
#[cfg(feature = "shared-array-buffer")]
pub type SharedArrayBufferIndex<'a> = BaseIndex<'a, SharedArrayBufferHeapData>;
pub type StringIndex<'a> = BaseIndex<'a, StringHeapData>;
pub type SymbolIndex<'a> = BaseIndex<'a, SymbolHeapData>;
#[cfg(feature = "array-buffer")]
pub type TypedArrayIndex<'a> = BaseIndex<'a, TypedArrayHeapData>;
#[cfg(feature = "weak-refs")]
pub type WeakMapIndex<'a> = BaseIndex<'a, WeakMapHeapData>;
#[cfg(feature = "weak-refs")]
pub type WeakRefIndex<'a> = BaseIndex<'a, WeakRefHeapData>;
#[cfg(feature = "weak-refs")]
pub type WeakSetIndex<'a> = BaseIndex<'a, WeakSetHeapData>;

impl TypedArrayIndex<'_> {
    /// Unbind this TypedArrayIndex from its current lifetime. This is necessary to use
    /// the TypedArrayIndex as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> TypedArrayIndex<'static> {
        unsafe { std::mem::transmute::<Self, TypedArrayIndex<'static>>(self) }
    }

    // Bind this TypedArrayIndex to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your TypedArrayIndexes cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let ta_idx = ta_idx.bind(&gc);
    // ```
    // to make sure that the unbound TypedArrayIndex cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> TypedArrayIndex<'gc> {
        unsafe { std::mem::transmute::<Self, TypedArrayIndex<'gc>>(self) }
    }
}

// Implement Default for ElementIndex: This is done to support Default
// constructor of ElementsVector.
impl Default for ElementIndex {
    fn default() -> Self {
        Self(
            unsafe { NonZeroU32::new_unchecked(1) },
            PhantomData,
            PhantomData,
        )
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
