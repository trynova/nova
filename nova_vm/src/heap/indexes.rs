// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{PropertyKey, Value},
    engine::context::{Bindable, GcToken, NoGcScope},
};
use core::fmt::Debug;
use core::{
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};
use core::{marker::PhantomData, mem::size_of, num::NonZeroU32};
use std::{any::type_name, u32};

/// A struct containing a non-zero index into an array or
/// vector of `T`s. Due to the non-zero value, the offset
/// in the vector is offset by one.
///
/// This index implies a tracing reference count from this
/// struct to T at the given index.
#[repr(transparent)]
pub struct BaseIndex<'a, T: ?Sized>(NonZeroU32, PhantomData<T>, PhantomData<&'a GcToken>);

// SAFETY: Marker lifetime transmute.
unsafe impl<T: ?Sized> Bindable for BaseIndex<'_, T> {
    type Of<'a> = BaseIndex<'a, T>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

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
    /// First valid BaseIndex.
    pub(crate) const ZERO: Self = Self(NonZeroU32::new(1).unwrap(), PhantomData, PhantomData);
    pub(crate) const MAX: Self = Self(NonZeroU32::new(u32::MAX).unwrap(), PhantomData, PhantomData);

    #[inline(always)]
    pub(crate) fn last(vec: &[T]) -> Self
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
pub type PropertyKeyIndex<'a> = BaseIndex<'a, [PropertyKey<'static>]>;

// Implement Default for ElementIndex: This is done to support Default
// constructor of ElementsVector.
impl Default for ElementIndex<'static> {
    #[inline(always)]
    fn default() -> Self {
        Self(
            const { NonZeroU32::new(1).unwrap() },
            PhantomData,
            PhantomData,
        )
    }
}

impl ElementIndex<'_> {
    #[inline(always)]
    pub fn last_element_index<const N: usize>(vec: &[[Option<Value>; N]]) -> Self {
        assert!(!vec.is_empty());
        Self::from_usize(vec.len())
    }
}

impl<const N: usize> Index<ElementIndex<'_>> for Vec<[Option<Value<'static>>; N]> {
    type Output = [Option<Value<'static>>; N];

    fn index(&self, index: ElementIndex) -> &Self::Output {
        self.get(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
    }
}

impl<const N: usize> IndexMut<ElementIndex<'_>> for Vec<[Option<Value<'static>>; N]> {
    fn index_mut(&mut self, index: ElementIndex<'_>) -> &mut Self::Output {
        self.get_mut(index.into_index())
            .expect("Invalid ElementsVector: No item at index")
    }
}

impl Default for PropertyKeyIndex<'static> {
    fn default() -> Self {
        Self(
            const { NonZeroU32::new(1).unwrap() },
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

/// Trait for working with index-based heap handles. The handles are internally
/// limited to 32 bit unsigned values.
pub(crate) trait HeapIndexHandle: Copy + Sized {
    /// Constant-time value used for discriminant checking only.
    const _DEF: Self;

    /// Convert an index into a heap handle.
    fn from_index(index: usize) -> Self {
        Self::from_index_u32(
            u32::try_from(index).expect(&format!("{} index out of bounds", type_name::<Self>())),
        )
    }
    /// Convert a 32-bit index into a heap handle.
    fn from_index_u32(index: u32) -> Self;

    /// Get the handle's stored index.
    fn get_index(self) -> usize {
        self.get_index_u32() as usize
    }

    /// Get the handle's stored 32-bit index.
    fn get_index_u32(self) -> u32;
}

impl<T: ?Sized> HeapIndexHandle for BaseIndex<'_, T> {
    const _DEF: Self = Self(NonZeroU32::new(u32::MAX).unwrap(), PhantomData, PhantomData);

    #[inline(always)]
    fn from_index_u32(index: u32) -> Self {
        assert!(index != u32::MAX);
        // SAFETY: Number is not max value and will not overflow to zero.
        // This check is done manually to allow const context.
        Self(
            unsafe { NonZeroU32::new_unchecked(index + 1) },
            PhantomData,
            PhantomData,
        )
    }

    #[inline(always)]
    fn get_index_u32(self) -> u32 {
        self.0.get() - 1
    }
}
