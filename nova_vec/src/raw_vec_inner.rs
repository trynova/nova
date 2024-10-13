use core::fmt;
use std::{
    alloc::{alloc_zeroed, handle_alloc_error, realloc, Layout, LayoutError},
    cmp, hint,
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
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
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
    pub fn new(ptr: *mut T) -> Option<Self> {
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
    pub const unsafe fn as_ref(&self) -> &T {
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
    pub unsafe fn as_mut(&mut self) -> &mut T {
        // SAFETY: the caller must guarantee that `self` meets all the
        // requirements for a mutable reference.
        unsafe { self.pointer.as_mut() }
    }

    /// Casts to a pointer of another type.
    #[must_use = "`self` will be dropped if the result is not used"]
    #[inline]
    pub const fn cast<U>(self) -> Unique<U> {
        // FIXME(const-hack): replace with `From`
        // SAFETY: is `NonNull`
        unsafe { Unique::new_unchecked(self.pointer.cast().as_ptr()) }
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

#[repr(C, packed)]
pub(crate) struct NovaRawVecInner {
    ptr: Unique<u8>,
    cap: u32,
}

impl NovaRawVecInner {
    #[must_use]
    pub(crate) const fn new<T>() -> Self {
        let align = core::mem::align_of::<T>();
        if align <= u32::MAX as usize {
            Self::new_in(core::mem::align_of::<T>())
        } else {
            panic!("Struct align is too large")
        }
    }

    #[inline]
    const fn new_in(align: usize) -> Self {
        let ptr = unsafe { core::mem::transmute(align) };
        // `cap: 0` means "unallocated". zero-sized types are ignored.
        Self { ptr, cap: 0 }
    }

    #[must_use]
    #[inline]
    pub(crate) fn with_capacity(capacity: u32, elem_layout: Layout) -> Self {
        let Ok(layout) = layout_array(capacity, elem_layout) else {
            capacity_overflow()
        };

        // Don't allocate here because `Drop` will not deallocate when `capacity` is 0.
        if layout.size() == 0 {
            return Self::new_in(elem_layout.align());
        }

        if let Err(_) = alloc_guard(layout.size()) {
            capacity_overflow()
        }

        // SAFETY: Checked size and alignment.
        let result = unsafe { alloc_zeroed(layout) };
        let ptr = match NonNull::new(result) {
            Some(ptr) => ptr,
            None => handle_alloc_error(layout),
        };

        // Allocators currently return a `NonNull<[u8]>` whose length
        // matches the size requested. If that ever changes, the capacity
        // here should change to `ptr.len() / mem::size_of::<T>()`.
        Self {
            ptr: Unique::from(ptr.cast()),
            cap: capacity,
        }
    }

    #[cold]
    pub(crate) fn grow_amortized(&mut self, len: u32, additional: u32, elem_layout: Layout) {
        // This is ensured by the calling contexts.
        debug_assert!(additional > 0);

        if elem_layout.size() == 0 {
            // Since we return a capacity of `usize::MAX` when `elem_size` is
            // 0, getting to here necessarily means the `RawVec` is overfull.
            capacity_overflow();
        }

        // Nothing we can really do about these checks, sadly.
        let Some(required_cap) = len.checked_add(additional) else {
            capacity_overflow()
        };

        // This guarantees exponential growth. The doubling cannot overflow
        // because `cap <= isize::MAX` and the type of `cap` is `usize`.
        let cap = cmp::max(self.cap * 2, required_cap);
        let cap = cmp::max(min_non_zero_cap(elem_layout.size()), cap);

        let Ok(new_layout) = layout_array(cap, elem_layout) else {
            capacity_overflow()
        };

        let Some(ptr) = finish_grow(new_layout, self.current_memory(elem_layout)) else {
            capacity_overflow()
        };
        // SAFETY: finish_grow would have resulted in a capacity overflow if we tried to allocate more than `isize::MAX` items

        unsafe { self.set_ptr_and_cap(ptr, cap) };
    }

    #[inline]
    unsafe fn set_ptr_and_cap(&mut self, ptr: NonNull<u8>, cap: u32) {
        // Allocators currently return a `NonNull<[u8]>` whose length matches
        // the size requested. If that ever changes, the capacity here should
        // change to `ptr.len() / mem::size_of::<T>()`.
        self.ptr = Unique::from(ptr.cast());
        self.cap = cap;
    }

    #[inline]
    fn current_memory(&self, elem_layout: Layout) -> Option<(NonNull<u8>, Layout)> {
        if elem_layout.size() == 0 || self.cap == 0 {
            None
        } else {
            // We could use Layout::array here which ensures the absence of isize and usize overflows
            // and could hypothetically handle differences between stride and size, but this memory
            // has already been allocated so we know it can't overflow and currently Rust does not
            // support such types. So we can do better by skipping some checks and avoid an unwrap.
            unsafe {
                let alloc_size = elem_layout.size().unchecked_mul(self.cap as usize);
                let layout = Layout::from_size_align_unchecked(alloc_size, elem_layout.align());
                Some((self.ptr.pointer, layout))
            }
        }
    }

    /// # Safety
    ///
    /// This function deallocates the owned allocation, but does not update `ptr` or `cap` to
    /// prevent double-free or use-after-free. Essentially, do not do anything with the caller
    /// after this function returns.
    /// Ideally this function would take `self` by move, but it cannot because it exists to be
    /// called from a `Drop` impl.
    pub(crate) unsafe fn deallocate(&mut self, elem_layout: Layout) {
        if let Some((mut ptr, layout)) = self.current_memory(elem_layout) {
            unsafe {
                std::alloc::dealloc(ptr.as_mut(), layout);
            }
        }
    }

    #[inline]
    pub(crate) const fn capacity(&self) -> u32 {
        self.cap
    }
}

#[inline]
fn layout_array(cap: u32, elem_layout: Layout) -> Result<Layout, LayoutError> {
    Layout::from_size_align(elem_layout.size() * cap as usize, elem_layout.align())
}

#[inline(never)]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

#[inline]
fn alloc_guard(alloc_size: usize) -> Result<(), ()> {
    if usize::BITS < 64 && alloc_size > isize::MAX as usize {
        Err(())
    } else {
        Ok(())
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

#[inline(never)]
fn finish_grow(
    new_layout: Layout,
    current_memory: Option<(NonNull<u8>, Layout)>,
) -> Option<NonNull<u8>> {
    if alloc_guard(new_layout.size()).is_err() {
        return None;
    }

    let memory = if let Some((mut ptr, old_layout)) = current_memory {
        debug_assert_eq!(old_layout.align(), new_layout.align());
        unsafe {
            // The allocator checks for alignment equality
            hint::assert_unchecked(old_layout.align() == new_layout.align());
            realloc(ptr.as_mut(), old_layout, new_layout.size())
        }
    } else {
        unsafe { std::alloc::alloc(new_layout) }
    };

    NonNull::new(memory)
}
