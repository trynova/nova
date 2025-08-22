use core::{alloc::Layout, marker::PhantomData};
use std::{cmp, ptr::NonNull};

use crate::{
    raw_vec_inner::{AllocError, RawSoAVecInner},
    soable::{SoATuple, SoAble},
};

#[repr(C)]
pub(crate) struct RawSoAVec<T: SoAble> {
    inner: RawSoAVecInner,
    cap: u32,
    len: u32,
    marker: PhantomData<T::TupleRepr>,
}

unsafe impl<T: SoAble + Send + Sized> Send for RawSoAVec<T> {}
unsafe impl<T: SoAble + Sync + Sized> Sync for RawSoAVec<T> {}

impl<T: SoAble> Drop for RawSoAVec<T> {
    fn drop(&mut self) {
        // SAFETY: Drop
        unsafe {
            let capacity = self.capacity();
            if capacity > 0 {
                self.inner
                    .deallocate(T::TupleRepr::layout(capacity).unwrap_unchecked())
            }
        };
    }
}

impl<T: SoAble> RawSoAVec<T> {
    #[inline(always)]
    pub(crate) fn capacity(&self) -> u32 {
        self.cap
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    #[inline(always)]
    pub(crate) fn set_len(&mut self, len: u32) {
        self.len = len
    }

    pub(crate) fn with_capacity(capacity: u32) -> Result<Self, AllocError> {
        Ok(Self {
            inner: RawSoAVecInner::with_layout(
                T::TupleRepr::layout(capacity).map_err(AllocError::LayoutError)?,
            )?,
            cap: capacity,
            len: 0,
            marker: PhantomData,
        })
    }

    pub fn reserve(&mut self, additional: u32) -> Result<(), AllocError> {
        if self.needs_to_grow(additional) {
            self.grow_amortized(additional)?;
        }
        Ok(())
    }

    #[cold]
    fn grow_amortized(&mut self, additional: u32) -> Result<(), AllocError> {
        let len = self.len();
        // This is ensured by the calling contexts.
        debug_assert!(additional > 0);

        // Nothing we can really do about these checks, sadly.
        let Some(required_cap) = len.checked_add(additional) else {
            return Err(AllocError::CapacityOverflow);
        };

        // This guarantees exponential growth.
        let cap = cmp::max(self.capacity().saturating_mul(2), required_cap);
        let cap = cmp::max(
            min_non_zero_cap(T::TupleRepr::layout(1).unwrap().size()),
            cap,
        );

        let new_layout = T::TupleRepr::layout(cap).map_err(AllocError::LayoutError)?;

        if new_layout.size() == 0 {
            // Since we return a capacity of `usize::MAX` when `elem_size` is
            // 0, getting to here necessarily means the `RawVec` is overfull.
            return Err(AllocError::CapacityOverflow);
        }

        let old_cap = self.capacity();
        self.inner
            .grow_amortized_inner(new_layout, self.current_memory())?;
        unsafe { T::TupleRepr::grow(self.inner.ptr(), cap, old_cap) };
        self.cap = cap;
        Ok(())
    }

    #[inline]
    fn current_memory(&self) -> Option<Layout> {
        if self.capacity() == 0 {
            None
        } else {
            // SAFETY: this layout has already been allocated.
            unsafe {
                let layout = T::TupleRepr::layout(self.capacity()).unwrap_unchecked();
                let alloc_size = layout.size();
                Some(Layout::from_size_align_unchecked(
                    alloc_size,
                    layout.align(),
                ))
            }
        }
    }

    fn needs_to_grow(&self, additional: u32) -> bool {
        additional > self.capacity().wrapping_sub(self.len)
    }

    pub(crate) fn as_ptr(&self) -> NonNull<u8> {
        self.inner.ptr()
    }

    pub(crate) fn as_mut_ptr(&mut self) -> NonNull<u8> {
        self.inner.ptr()
    }
}

const fn min_non_zero_cap(size: usize) -> u32 {
    if size == 1 {
        8
    } else if size <= 1024 {
        4
    } else {
        1
    }
}
