use std::{alloc::Layout, cmp, marker::PhantomData, mem::MaybeUninit, ptr::NonNull};

use crate::raw_vec_inner::NovaRawVecInner;

#[repr(transparent)]
// Lang item used experimentally by Miri to define the semantics of `Unique`.
struct Unique<T: ?Sized> {
    pointer: NonNull<T>,
    // NOTE: this marker has no consequences for variance, but is necessary
    // for dropck to understand that we logically own a `T`.
    //
    // For details, see:
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    _marker: PhantomData<T>,
}

pub(crate) struct NovaVecData2<T: Sized, U: Sized> {
    first: [MaybeUninit<T>; 0],
    second: [MaybeUninit<U>; 0],
}

impl<T, U> NovaVecData2<T, U> {
    fn layout(count: u32) -> Layout {
        assert_eq!(Self::CORRECTLY_ORDERED, ());
        Layout::array::<T>(count as usize)
            .unwrap()
            .pad_to_align()
            .extend(Layout::array::<U>(count as usize).unwrap())
            .unwrap()
            .0
            .pad_to_align()
    }

    const CORRECTLY_ORDERED: () = assert!(
        std::mem::align_of::<T>() >= std::mem::align_of::<U>(),
        "Order fields in falling order of alignment to avoid unnecessary padding"
    );
}

#[repr(C)]
pub(crate) struct NovaRawVec2<T: Sized, U: Sized> {
    inner: NovaRawVecInner,
    marker: PhantomData<(T, U)>,
}

unsafe impl<T: Send + Sized, U: Send + Sized> Send for NovaRawVec2<T, U> {}
unsafe impl<T: Sync + Sized, U: Sync + Sized> Sync for NovaRawVec2<T, U> {}

impl<T, U> Drop for NovaRawVec2<T, U> {
    fn drop(&mut self) {
        // SAFETY: Drop
        unsafe { self.inner.deallocate(Self::ELEM_LAYOUT) };
    }
}

impl<T, U> NovaRawVec2<T, U> {
    /// Layout that defines the "single element layout" that is never seen in
    /// the Vec2 data. Because element parts are forced to appear in order of
    /// alignment we know that the size of ([T; N], [U; N]) is exactly equal to
    /// [(T, U); N].
    pub(crate) const ELEM_LAYOUT: Layout = {
        assert!(
            size_of::<T>() > 0 && size_of::<U>() > 0,
            "ZST element parts are not supported"
        );
        assert!(
            align_of::<T>() >= align_of::<U>(),
            "Element parts must be defined in alignment order"
        );
        assert!(
            size_of::<(T, U)>() > 0
                && (size_of::<(T, U)>().next_multiple_of(align_of::<(T, U)>())
                    == size_of::<(T, U)>())
        );
        assert!(align_of::<(T, U)>() > 0 && align_of::<(T, U)>().is_power_of_two());
        unsafe { Layout::from_size_align_unchecked(size_of::<(T, U)>(), align_of::<(T, U)>()) }
    };
    pub(crate) const NEW: Self = NovaRawVec2 {
        inner: NovaRawVecInner::new::<(T, U)>(),
        marker: PhantomData,
    };

    pub(crate) fn capacity(&self) -> u32 {
        assert!(size_of::<T>() > 0 && size_of::<U>() > 0);
        self.inner.capacity()
    }

    pub(crate) fn with_capacity(capacity: u32) -> Self {
        Self {
            inner: NovaRawVecInner::with_capacity(capacity, Self::ELEM_LAYOUT),
            marker: PhantomData,
        }
    }

    pub fn reserve(&mut self, len: u32, additional: u32) {
        if self.needs_to_grow(len, additional) {
            self.inner
                .grow_amortized(len, additional, Self::ELEM_LAYOUT);
        }
    }

    fn needs_to_grow(&self, len: u32, additional: u32) -> bool {
        additional > self.capacity().wrapping_sub(len)
    }
}
