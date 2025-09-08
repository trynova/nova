// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "array-buffer")]
use crate::ecmascript::builtins::typed_array::data::TypedArrayHeapData;
use crate::{
    ecmascript::types::{PropertyKey, Value},
    engine::context::{GcToken, bindable_handle},
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
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        assert!(self.0.get() != 0);
        (&self.0.get() - 1).fmt(f)
    }
}

impl<T: ?Sized> Clone for BaseIndex<'_, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for BaseIndex<'_, T> {}

impl<T: ?Sized> PartialEq for BaseIndex<'_, T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<T: ?Sized> Eq for BaseIndex<'_, T> {}

impl<T: ?Sized> PartialOrd for BaseIndex<'_, T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for BaseIndex<'_, T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: ?Sized> Hash for BaseIndex<'_, T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> BaseIndex<'_, T> {
    #[inline(always)]
    pub const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }

    #[inline(always)]
    pub const fn into_u32_index(self) -> u32 {
        self.0.get() - 1
    }

    #[inline(always)]
    pub const fn into_usize(self) -> usize {
        self.0.get() as usize
    }

    #[inline(always)]
    pub const fn into_u32(self) -> u32 {
        self.0.get()
    }

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
    pub fn last(vec: &[Option<T>]) -> Self
    where
        T: Sized,
    {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }

    #[inline(always)]
    pub(crate) fn last_t(vec: &[T]) -> Self
    where
        T: Sized,
    {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

impl<T> Default for BaseIndex<'_, T> {
    #[inline(always)]
    fn default() -> Self {
        Self::from_u32_index(0)
    }
}

pub type ElementIndex<'a> = BaseIndex<'a, [Option<Value<'static>>]>;
bindable_handle!(ElementIndex);
pub type PropertyKeyIndex<'a> = BaseIndex<'a, [PropertyKey<'static>]>;
bindable_handle!(PropertyKeyIndex);

#[cfg(feature = "array-buffer")]
pub type TypedArrayIndex<'a> = BaseIndex<'a, TypedArrayHeapData<'static>>;
#[cfg(feature = "array-buffer")]
bindable_handle!(TypedArrayIndex);

// Implement Default for ElementIndex: This is done to support Default
// constructor of ElementsVector.
impl Default for ElementIndex<'static> {
    #[inline(always)]
    fn default() -> Self {
        Self(
            unsafe { NonZeroU32::new_unchecked(1) },
            PhantomData,
            PhantomData,
        )
    }
}

impl ElementIndex<'_> {
    #[inline(always)]
    pub fn last_element_index<const N: usize>(vec: &[Option<[Option<Value>; N]>]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
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
    #[inline(always)]
    pub fn last_property_key_index<const N: usize>(vec: &[[Option<PropertyKey>; N]]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}
