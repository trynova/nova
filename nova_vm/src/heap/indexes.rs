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
    ArrayBufferHeapData, data_view::data::DataViewHeapData, typed_array::data::TypedArrayHeapData,
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
            ArrayHeapData, async_generator_objects::AsyncGeneratorHeapData,
            control_abstraction_objects::generator_objects::GeneratorHeapData,
            embedder_object::data::EmbedderObjectHeapData, error::ErrorHeapData,
            finalization_registry::data::FinalizationRegistryHeapData,
            indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIteratorHeapData,
            keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIteratorHeapData,
            map::data::MapHeapData, primitive_objects::PrimitiveObjectHeapData,
            promise::data::PromiseHeapData, proxy::data::ProxyHeapData,
            text_processing::string_objects::string_iterator_objects::StringIteratorHeapData,
        },
        types::{
            BigIntHeapData, BoundFunctionHeapData, BuiltinConstructorHeapData,
            BuiltinFunctionHeapData, ECMAScriptFunctionHeapData, NumberHeapData, ObjectHeapData,
            PropertyKey, StringHeapData, SymbolHeapData, Value,
        },
    },
    engine::context::{Bindable, GcToken, NoGcScope},
};
use core::fmt::Debug;
use core::{
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};
use core::{marker::PhantomData, mem::size_of, num::NonZeroU32};

/// A struct containing a non-zero index into an array or
/// vector of `T`s. Due to the non-zero value, the offset
/// in the vector is offset by one.
///
/// This index implies a tracing reference count from this
/// struct to T at the given index.
#[repr(transparent)]
pub struct BaseIndex<'a, T: ?Sized>(NonZeroU32, PhantomData<T>, PhantomData<&'a GcToken>);

const _INDEX_SIZE_IS_U32: () = assert!(size_of::<BaseIndex<()>>() == size_of::<u32>());
const _OPTION_INDEX_SIZE_IS_U32: () =
    assert!(size_of::<Option<BaseIndex<()>>>() == size_of::<u32>());

impl<T: ?Sized> Debug for BaseIndex<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        assert!(self.0.get() != 0);
        (&self.0.get() - 1).fmt(f)
    }
}

impl<T: ?Sized> Clone for BaseIndex<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for BaseIndex<'_, T> {}

impl<T: ?Sized> PartialEq for BaseIndex<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Eq for BaseIndex<'_, T> {}

impl<T: ?Sized> PartialOrd for BaseIndex<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for BaseIndex<'_, T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Hash for BaseIndex<'_, T> {
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

    pub const fn from_index(value: usize) -> Self {
        let value = value as u32;
        assert!(value != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(
            unsafe { NonZeroU32::new_unchecked(value.wrapping_add(1)) },
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

impl<T> Default for BaseIndex<'_, T> {
    fn default() -> Self {
        Self::from_u32_index(0)
    }
}

#[cfg(feature = "array-buffer")]
pub type ArrayBufferIndex<'a> = BaseIndex<'a, ArrayBufferHeapData<'static>>;
pub type ArrayIndex<'a> = BaseIndex<'a, ArrayHeapData<'static>>;
pub type ArrayIteratorIndex<'a> = BaseIndex<'a, ArrayIteratorHeapData<'static>>;
pub type AsyncGeneratorIndex<'a> = BaseIndex<'a, AsyncGeneratorHeapData<'static>>;
pub type BigIntIndex<'a> = BaseIndex<'a, BigIntHeapData>;
pub type BoundFunctionIndex<'a> = BaseIndex<'a, BoundFunctionHeapData<'static>>;
pub type BuiltinFunctionIndex<'a> = BaseIndex<'a, BuiltinFunctionHeapData<'static>>;
pub type BuiltinConstructorIndex<'a> = BaseIndex<'a, BuiltinConstructorHeapData<'static>>;
#[cfg(feature = "array-buffer")]
pub type DataViewIndex<'a> = BaseIndex<'a, DataViewHeapData<'static>>;
#[cfg(feature = "date")]
pub type DateIndex<'a> = BaseIndex<'a, DateHeapData<'static>>;
pub type ECMAScriptFunctionIndex<'a> = BaseIndex<'a, ECMAScriptFunctionHeapData<'static>>;
pub type ElementIndex<'a> = BaseIndex<'a, [Option<Value<'static>>]>;
pub type PropertyKeyIndex<'a> = BaseIndex<'a, [PropertyKey<'static>]>;
pub type EmbedderObjectIndex<'a> = BaseIndex<'a, EmbedderObjectHeapData>;
pub type ErrorIndex<'a> = BaseIndex<'a, ErrorHeapData<'static>>;
pub type FinalizationRegistryIndex<'a> = BaseIndex<'a, FinalizationRegistryHeapData<'static>>;
pub type GeneratorIndex<'a> = BaseIndex<'a, GeneratorHeapData<'static>>;
pub type StringIteratorIndex<'a> = BaseIndex<'a, StringIteratorHeapData<'static>>;
pub type MapIndex<'a> = BaseIndex<'a, MapHeapData<'static>>;
pub type MapIteratorIndex<'a> = BaseIndex<'a, MapIteratorHeapData<'static>>;
pub type NumberIndex<'a> = BaseIndex<'a, NumberHeapData>;
pub type ObjectIndex<'a> = BaseIndex<'a, ObjectHeapData<'static>>;
pub type PrimitiveObjectIndex<'a> = BaseIndex<'a, PrimitiveObjectHeapData<'static>>;
pub type PromiseIndex<'a> = BaseIndex<'a, PromiseHeapData<'static>>;
pub type ProxyIndex<'a> = BaseIndex<'a, ProxyHeapData<'static>>;
#[cfg(feature = "regexp")]
pub type RegExpIndex<'a> = BaseIndex<'a, RegExpHeapData<'static>>;
#[cfg(feature = "set")]
pub type SetIndex<'a> = BaseIndex<'a, SetHeapData<'static>>;
#[cfg(feature = "set")]
pub type SetIteratorIndex<'a> = BaseIndex<'a, SetIteratorHeapData<'static>>;
#[cfg(feature = "shared-array-buffer")]
pub type SharedArrayBufferIndex<'a> = BaseIndex<'a, SharedArrayBufferHeapData<'static>>;
pub type StringIndex<'a> = BaseIndex<'a, StringHeapData>;
pub type SymbolIndex<'a> = BaseIndex<'a, SymbolHeapData<'static>>;
#[cfg(feature = "array-buffer")]
pub type TypedArrayIndex<'a> = BaseIndex<'a, TypedArrayHeapData<'static>>;
#[cfg(feature = "weak-refs")]
pub type WeakMapIndex<'a> = BaseIndex<'a, WeakMapHeapData<'static>>;
#[cfg(feature = "weak-refs")]
pub type WeakRefIndex<'a> = BaseIndex<'a, WeakRefHeapData<'static>>;
#[cfg(feature = "weak-refs")]
pub type WeakSetIndex<'a> = BaseIndex<'a, WeakSetHeapData<'static>>;

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for TypedArrayIndex<'_> {
    type Of<'a> = TypedArrayIndex<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

// Implement Default for ElementIndex: This is done to support Default
// constructor of ElementsVector.
impl Default for ElementIndex<'static> {
    fn default() -> Self {
        Self(
            unsafe { NonZeroU32::new_unchecked(1) },
            PhantomData,
            PhantomData,
        )
    }
}

impl ElementIndex<'_> {
    pub fn last_element_index<const N: usize>(vec: &[Option<[Option<Value>; N]>]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ElementIndex<'_> {
    type Of<'a> = ElementIndex<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<const N: usize> Index<ElementIndex<'_>> for Vec<Option<[Option<Value<'static>>; N]>> {
    type Output = [Option<Value<'static>>; N];

    fn index(&self, index: ElementIndex) -> &Self::Output {
        self.get(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
            .as_ref()
            .expect("Invalid ElementsVector: Found None at index")
    }
}

impl<const N: usize> IndexMut<ElementIndex<'_>> for Vec<Option<[Option<Value<'static>>; N]>> {
    fn index_mut(&mut self, index: ElementIndex<'_>) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
            .as_mut()
            .expect("Invalid ElementsVector: Found None at index")
    }
}

impl Default for PropertyKeyIndex<'static> {
    fn default() -> Self {
        Self(
            unsafe { NonZeroU32::new_unchecked(1) },
            PhantomData,
            PhantomData,
        )
    }
}

impl PropertyKeyIndex<'_> {
    pub fn last_property_key_index<const N: usize>(vec: &[[Option<PropertyKey>; N]]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for PropertyKeyIndex<'_> {
    type Of<'a> = PropertyKeyIndex<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}
