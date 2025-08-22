use core::fmt;
use std::{
    alloc::{alloc_zeroed, realloc, Layout, LayoutError},
    hint,
    marker::PhantomData,
    ptr::NonNull,
};

#[repr(transparent)]
struct Unique<T: ?Sized> {
    pointer: NonNull<T>,
    // NOTE: this marker has no consequences for variance, but is necessary
    // for dropck to understand that we logically own a `T`.
    //
    // For details, see:
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    _marker: PhantomData<T>,
}

impl<T: ?Sized> Unique<T> {
    /// Creates a new `Unique`.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null.
    #[inline]
    pub const unsafe fn _new_unchecked(ptr: *mut T) -> Self {
        // SAFETY: the caller must guarantee that `ptr` is non-null.
        unsafe {
            Unique {
                pointer: NonNull::new_unchecked(ptr),
                _marker: PhantomData,
            }
        }
    }

    /// Creates a new `Unique` if `ptr` is non-null.
    #[inline]
    pub fn _new(ptr: *mut T) -> Option<Self> {
        if let Some(pointer) = NonNull::new(ptr) {
            Some(Unique {
                pointer,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Acquires the underlying `*mut` pointer.
    #[must_use = "`self` will be dropped if the result is not used"]
    #[inline]
    pub const fn as_ptr(self) -> *mut T {
        self.pointer.as_ptr()
    }

    /// Dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&*my_ptr.as_ptr()`.
    #[must_use]
    #[inline]
    pub const unsafe fn _as_ref(&self) -> &T {
        // SAFETY: the caller must guarantee that `self` meets all the
        // requirements for a reference.
        unsafe { self.pointer.as_ref() }
    }

    /// Mutably dereferences the content.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use `&mut *my_ptr.as_ptr()`.
    #[must_use]
    #[inline]
    pub unsafe fn _as_mut(&mut self) -> &mut T {
        // SAFETY: the caller must guarantee that `self` meets all the
        // requirements for a mutable reference.
        unsafe { self.pointer.as_mut() }
    }

    /// Casts to a pointer of another type.
    #[must_use = "`self` will be dropped if the result is not used"]
    #[inline]
    pub const fn _cast<U>(self) -> Unique<U> {
        // FIXME(const-hack): replace with `From`
        // SAFETY: is `NonNull`
        unsafe { Unique::_new_unchecked(self.pointer.cast().as_ptr()) }
    }
}

impl<T: ?Sized> Clone for Unique<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Unique<T> {}

impl<T: ?Sized> fmt::Debug for Unique<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

impl<T: ?Sized> fmt::Pointer for Unique<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

impl<T: ?Sized> From<&mut T> for Unique<T> {
    /// Converts a `&mut T` to a `Unique<T>`.
    ///
    /// This conversion is infallible since references cannot be null.
    #[inline]
    fn from(reference: &mut T) -> Self {
        Self::from(NonNull::from(reference))
    }
}

impl<T: ?Sized> From<NonNull<T>> for Unique<T> {
    /// Converts a `NonNull<T>` to a `Unique<T>`.
    ///
    /// This conversion is infallible since `NonNull` cannot be null.
    #[inline]
    fn from(pointer: NonNull<T>) -> Self {
        Unique {
            pointer,
            _marker: PhantomData,
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub(crate) struct RawSoAVecInner {
    ptr: Unique<u8>,
}

#[derive(Debug, Clone)]
pub enum AllocError {
    CapacityOverflow,
    AllocationFailure,
    LayoutError(LayoutError),
}

impl RawSoAVecInner {
    // #[must_use]
    // pub(crate) const fn new<T>() -> Result<Self, AllocError> {
    //     let align = core::mem::align_of::<T>();
    //     if align <= u32::MAX as usize {
    //         Ok(Self::new_in(core::mem::align_of::<T>()))
    //     } else {
    //         Err(AllocError::CapacityOverflow)
    //     }
    // }

    #[inline]
    const fn new_in(align: usize) -> Self {
        let ptr = unsafe { core::mem::transmute(align) };
        // `cap: 0` means "unallocated". zero-sized types are ignored.
        Self { ptr }
    }

    #[must_use]
    #[inline]
    pub(crate) fn with_layout(layout: Layout) -> Result<Self, AllocError> {
        // Don't allocate here because `Drop` will not deallocate when `capacity` is 0.
        if layout.size() == 0 {
            return Ok(Self::new_in(layout.align()));
        }

        if let Err(_) = alloc_guard(layout.size()) {
            return Err(AllocError::AllocationFailure);
        }

        // SAFETY: Checked size and alignment.
        let result = unsafe { alloc_zeroed(layout) };
        let ptr = match NonNull::new(result) {
            Some(ptr) => ptr,
            None => return Err(AllocError::AllocationFailure),
        };

        // Allocators currently return a `NonNull<[u8]>` whose length
        // matches the size requested. If that ever changes, the capacity
        // here should change to `ptr.len() / mem::size_of::<T>()`.
        Ok(Self {
            ptr: Unique::from(ptr),
        })
    }

    // #[must_use]
    // #[inline]
    // fn _with_capacity(capacity: u32, elem_layout: Layout) -> Self {
    //     let Ok(layout) = layout_array(capacity, elem_layout) else {
    //         capacity_overflow()
    //     };

    //     // Don't allocate here because `Drop` will not deallocate when `capacity` is 0.
    //     if layout.size() == 0 {
    //         return Self::new_in(elem_layout.align());
    //     }

    //     if let Err(_) = alloc_guard(layout.size()) {
    //         capacity_overflow()
    //     }

    //     // SAFETY: Checked size and alignment.
    //     let result = unsafe { alloc_zeroed(layout) };
    //     let ptr = match NonNull::new(result) {
    //         Some(ptr) => ptr,
    //         None => handle_alloc_error(layout),
    //     };

    //     // Allocators currently return a `NonNull<[u8]>` whose length
    //     // matches the size requested. If that ever changes, the capacity
    //     // here should change to `ptr.len() / mem::size_of::<T>()`.
    //     Self {
    //         ptr: Unique::from(ptr.cast()),
    //         cap: capacity,
    //     }
    // }

    #[cold]
    pub(crate) fn grow_amortized_inner(
        &mut self,
        new_layout: Layout,
        old_layout: Option<Layout>,
    ) -> Result<(), AllocError> {
        if new_layout.size() == 0 {
            // Since we return a capacity of `u32::MAX` when `elem_size` is
            // 0, getting to here necessarily means the `RawVec` is overfull.
            return Err(AllocError::CapacityOverflow);
        }

        let Some(ptr) = finish_grow(new_layout, old_layout.map(|l| (self.ptr.pointer, l))) else {
            return Err(AllocError::AllocationFailure);
        };
        // SAFETY: finish_grow would have resulted in a capacity overflow if we
        // tried to allocate more than `isize::MAX` items

        unsafe { self.set_ptr(ptr) };
        Ok(())
    }

    #[inline]
    unsafe fn set_ptr(&mut self, ptr: NonNull<u8>) {
        // Allocators currently return a `NonNull<[u8]>` whose length matches
        // the size requested. If that ever changes, the capacity here should
        // change to `ptr.len() / mem::size_of::<T>()`.
        self.ptr = Unique::from(ptr.cast());
    }

    /// # Safety
    ///
    /// This function deallocates the owned allocation, but does not update `ptr` or `cap` to
    /// prevent double-free or use-after-free. Essentially, do not do anything with the caller
    /// after this function returns.
    /// Ideally this function would take `self` by move, but it cannot because it exists to be
    /// called from a `Drop` impl.
    pub(crate) unsafe fn deallocate(&mut self, layout: Layout) {
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr(), layout);
        }
    }

    #[inline]
    pub(crate) const fn ptr(&self) -> NonNull<u8> {
        self.ptr.pointer
    }
}

#[inline]
fn alloc_guard(alloc_size: usize) -> Result<(), ()> {
    if usize::BITS < 64 && alloc_size > isize::MAX as usize {
        Err(())
    } else {
        Ok(())
    }
}

#[inline(never)]
fn finish_grow(
    new_layout: Layout,
    current_memory: Option<(NonNull<u8>, Layout)>,
) -> Option<NonNull<u8>> {
    if alloc_guard(new_layout.size()).is_err() {
        return None;
    }

    let memory = if let Some((ptr, old_layout)) = current_memory {
        debug_assert_eq!(old_layout.align(), new_layout.align());
        unsafe {
            // The allocator checks for alignment equality
            hint::assert_unchecked(old_layout.align() == new_layout.align());
            realloc(ptr.as_ptr(), old_layout, new_layout.size())
        }
    } else {
        unsafe { std::alloc::alloc(new_layout) }
    };

    NonNull::new(memory)
}
