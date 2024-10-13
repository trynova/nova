use core::marker::PhantomData;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr::{self, NonNull};
use core::{cmp, hint};

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

use std::alloc::{alloc, Layout};
use std::alloc::{handle_alloc_error, realloc};
use std::boxed::Box;
use std::collections::TryReserveError;

// One central function responsible for reporting capacity overflows. This'll
// ensure that the code generation related to these panics is minimal as there's
// only one location which panics rather than a bunch throughout the module.
#[inline(never)]
fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

struct Cap(u32);

impl Cap {
    const ZERO: Cap = Cap(0);

    /// `Cap(cap)`, except if `T` is a ZST then `Cap::ZERO`.
    ///
    /// # Safety: cap must be <= `isize::MAX`.
    unsafe fn new<T>(cap: u32) -> Self {
        if size_of::<T>() == 0 {
            Cap::ZERO
        } else {
            Self(cap)
        }
    }
}

/// A low-level utility for more ergonomically allocating, reallocating, and deallocating
/// a buffer of memory on the heap without having to worry about all the corner cases
/// involved. This type is excellent for building your own data structures like Vec and VecDeque.
/// In particular:
///
/// * Produces `Unique::dangling()` on zero-sized types.
/// * Produces `Unique::dangling()` on zero-length allocations.
/// * Avoids freeing `Unique::dangling()`.
/// * Catches all overflows in capacity computations (promotes them to "capacity overflow" panics).
/// * Guards against 32-bit systems allocating more than `isize::MAX` bytes.
/// * Guards against overflowing your length.
/// * Calls `handle_alloc_error` for fallible allocations.
/// * Contains a `ptr::Unique` and thus endows the user with all related benefits.
/// * Uses the excess returned from the allocator to use the largest available capacity.
///
/// This type does not in anyway inspect the memory that it manages. When dropped it *will*
/// free its memory, but it *won't* try to drop its contents. It is up to the user of `RawVec`
/// to handle the actual things *stored* inside of a `RawVec`.
///
/// Note that the excess of a zero-sized types is always infinite, so `capacity()` always returns
/// `usize::MAX`. This means that you need to be careful when round-tripping this type with a
/// `Box<[T]>`, since `capacity()` won't yield the length.
#[allow(missing_debug_implementations)]
pub(crate) struct RawVec<T> {
    inner: RawVecInner,
    _marker: PhantomData<T>,
}

/// Like a `RawVec` but not generic over the type.
///
/// As such, all the methods need the layout passed-in as a parameter.
///
/// Having this separation reduces the amount of code we need to monomorphize,
/// as most operations don't need the actual type, just its layout.
#[allow(missing_debug_implementations)]
struct RawVecInner {
    ptr: Unique<u8>,
    /// Never used for ZSTs; it's `capacity()`'s responsibility to return usize::MAX in that case.
    ///
    /// # Safety
    ///
    /// `cap` must be in the `0..=isize::MAX` range.
    cap: Cap,
}

impl RawVecInner {
    #[must_use]
    const fn new<T>() -> Self {
        let align = core::mem::align_of::<T>();
        if align <= u32::MAX as usize {
            Self::new_in(core::mem::align_of::<T>() as u32)
        } else {
            panic!("Struct align is too large")
        }
    }

    #[must_use]
    #[inline]
    fn with_capacity(capacity: u32, elem_layout: Layout) -> Self {
        Self::try_allocate_in(capacity, elem_layout)
    }
}

impl<T> RawVec<T> {
    /// HACK(Centril): This exists because stable `const fn` can only call stable `const fn`, so
    /// they cannot call `Self::new()`.
    ///
    /// If you change `RawVec<T>::new` or dependencies, please take care to not introduce anything
    /// that would truly const-call something unstable.
    pub const NEW: Self = Self::new();

    /// Creates the biggest possible `RawVec` (on the system heap)
    /// without allocating. If `T` has positive size, then this makes a
    /// `RawVec` with capacity `0`. If `T` is zero-sized, then it makes a
    /// `RawVec` with capacity `usize::MAX`. Useful for implementing
    /// delayed allocation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: RawVecInner::new::<T>(),
            _marker: PhantomData,
        }
    }

    /// Creates a `RawVec` (on the system heap) with exactly the
    /// capacity and alignment requirements for a `[T; capacity]`. This is
    /// equivalent to calling `RawVec::new` when `capacity` is `0` or `T` is
    /// zero-sized. Note that if `T` is zero-sized this means you will
    /// *not* get a `RawVec` with the requested capacity.
    ///
    /// Non-fallible version of `try_with_capacity`
    ///
    /// # Panics
    ///
    /// Panics if the requested capacity exceeds `isize::MAX` bytes.
    ///
    /// # Aborts
    ///
    /// Aborts on OOM.
    #[must_use]
    #[inline]
    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            inner: RawVecInner::with_capacity(capacity, T::LAYOUT),
            _marker: PhantomData,
        }
    }

    /// Like `with_capacity`, but guarantees the buffer is zeroed.
    #[must_use]
    #[inline]
    pub fn with_capacity_zeroed(capacity: u32) -> Self {
        Self {
            inner: RawVecInner::with_capacity_zeroed_in(capacity, T::LAYOUT),
            _marker: PhantomData,
        }
    }
}

// Tiny Vecs are dumb. Skip to:
// - 8 if the element size is 1, because any heap allocators is likely
//   to round up a request of less than 8 bytes to at least 8 bytes.
// - 4 if elements are moderate-sized (<= 1 KiB).
// - 1 otherwise, to avoid wasting too much space for very short Vecs.
const fn min_non_zero_cap(size: u32) -> u32 {
    if size == 1 {
        8
    } else if size <= 1024 {
        4
    } else {
        1
    }
}

impl<T> RawVec<T> {
    pub(crate) const MIN_NON_ZERO_CAP: u32 = min_non_zero_cap(size_of::<T>());

    /// Like `new`, but parameterized over the choice of allocator for
    /// the returned `RawVec`.
    #[inline]
    pub const fn new_in() -> Self {
        Self {
            inner: RawVecInner::new_in(align_of::<T>()),
            _marker: PhantomData,
        }
    }

    /// Like `with_capacity`, but parameterized over the choice of
    /// allocator for the returned `RawVec`.
    #[inline]
    pub fn with_capacity_in(capacity: u32) -> Self {
        Self {
            inner: RawVecInner::with_capacity_in(capacity, T::LAYOUT),
            _marker: PhantomData,
        }
    }

    /// Like `try_with_capacity`, but parameterized over the choice of
    /// allocator for the returned `RawVec`.
    #[inline]
    pub fn try_with_capacity_in(capacity: u32) -> Result<Self, TryReserveError> {
        match RawVecInner::try_with_capacity_in(capacity, T::LAYOUT) {
            Ok(inner) => Ok(Self {
                inner,
                _marker: PhantomData,
            }),
            Err(e) => Err(e),
        }
    }

    /// Like `with_capacity_zeroed`, but parameterized over the choice
    /// of allocator for the returned `RawVec`.
    #[inline]
    pub fn with_capacity_zeroed_in(capacity: u32) -> Self {
        Self {
            inner: RawVecInner::with_capacity_zeroed_in(capacity, T::LAYOUT),
            _marker: PhantomData,
        }
    }

    /// Converts the entire buffer into `Box<[MaybeUninit<T>]>` with the specified `len`.
    ///
    /// Note that this will correctly reconstitute any `cap` changes
    /// that may have been performed. (See description of type for details.)
    ///
    /// # Safety
    ///
    /// * `len` must be greater than or equal to the most recently requested capacity, and
    /// * `len` must be less than or equal to `self.capacity()`.
    ///
    /// Note, that the requested capacity and `self.capacity()` could differ, as
    /// an allocator could overallocate and return a greater memory block than requested.
    pub unsafe fn into_box(self, len: u32) -> Box<[MaybeUninit<T>], A> {
        // Sanity-check one half of the safety requirement (we cannot check the other half).
        debug_assert!(
            len <= self.capacity(),
            "`len` must be smaller than or equal to `self.capacity()`"
        );

        let me = ManuallyDrop::new(self);
        unsafe {
            let slice = ptr::slice_from_raw_parts_mut(me.ptr() as *mut MaybeUninit<T>, len);
            Box::from_raw_in(slice, ptr::read(&me.inner.alloc))
        }
    }

    /// Reconstitutes a `RawVec` from a pointer, capacity, and allocator.
    ///
    /// # Safety
    ///
    /// The `ptr` must be allocated (via the given allocator `alloc`), and with the given
    /// `capacity`.
    /// The `capacity` cannot exceed `isize::MAX` for sized types. (only a concern on 32-bit
    /// systems). For ZSTs capacity is ignored.
    /// If the `ptr` and `capacity` come from a `RawVec` created via `alloc`, then this is
    /// guaranteed.
    #[inline]
    pub unsafe fn from_raw_parts_in(ptr: *mut T, capacity: u32) -> Self {
        // SAFETY: Precondition passed to the caller
        unsafe {
            let ptr = ptr.cast();
            let capacity = Cap::new::<T>(capacity);
            Self {
                inner: RawVecInner::from_raw_parts_in(ptr, capacity, alloc),
                _marker: PhantomData,
            }
        }
    }

    /// A convenience method for hoisting the non-null precondition out of [`RawVec::from_raw_parts_in`].
    ///
    /// # Safety
    ///
    /// See [`RawVec::from_raw_parts_in`].
    #[inline]
    pub unsafe fn from_nonnull_in(ptr: NonNull<T>, capacity: u32) -> Self {
        // SAFETY: Precondition passed to the caller
        unsafe {
            let ptr = ptr.cast();
            let capacity = Cap::new::<T>(capacity);
            Self {
                inner: RawVecInner::from_nonnull_in(ptr, capacity, alloc),
                _marker: PhantomData,
            }
        }
    }

    /// Gets a raw pointer to the start of the allocation. Note that this is
    /// `Unique::dangling()` if `capacity == 0` or `T` is zero-sized. In the former case, you must
    /// be careful.
    #[inline]
    pub fn ptr(&self) -> *mut T {
        self.inner.ptr()
    }

    #[inline]
    pub fn non_null(&self) -> NonNull<T> {
        self.inner.non_null()
    }

    /// Gets the capacity of the allocation.
    ///
    /// This will always be `usize::MAX` if `T` is zero-sized.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity(size_of::<T>())
    }

    /// Returns a shared reference to the allocator backing this `RawVec`.
    #[inline]
    pub fn allocator(&self) -> &A {
        self.inner.allocator()
    }

    /// Ensures that the buffer contains at least enough space to hold `len +
    /// additional` elements. If it doesn't already have enough capacity, will
    /// reallocate enough space plus comfortable slack space to get amortized
    /// *O*(1) behavior. Will limit this behavior if it would needlessly cause
    /// itself to panic.
    ///
    /// If `len` exceeds `self.capacity()`, this may fail to actually allocate
    /// the requested space. This is not really unsafe, but the unsafe
    /// code *you* write that relies on the behavior of this function may break.
    ///
    /// This is ideal for implementing a bulk-push operation like `extend`.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    /// # Aborts
    ///
    /// Aborts on OOM.
    #[inline]
    pub fn reserve(&mut self, len: u32, additional: u32) {
        self.inner.reserve(len, additional, T::LAYOUT)
    }

    /// A specialized version of `self.reserve(len, 1)` which requires the
    /// caller to ensure `len == self.capacity()`.
    #[inline(never)]
    pub fn grow_one(&mut self) {
        self.inner.grow_one(T::LAYOUT)
    }

    /// The same as `reserve`, but returns on errors instead of panicking or aborting.
    pub fn try_reserve(&mut self, len: u32, additional: u32) -> Result<(), TryReserveError> {
        self.inner.try_reserve(len, additional, T::LAYOUT)
    }

    /// Ensures that the buffer contains at least enough space to hold `len +
    /// additional` elements. If it doesn't already, will reallocate the
    /// minimum possible amount of memory necessary. Generally this will be
    /// exactly the amount of memory necessary, but in principle the allocator
    /// is free to give back more than we asked for.
    ///
    /// If `len` exceeds `self.capacity()`, this may fail to actually allocate
    /// the requested space. This is not really unsafe, but the unsafe code
    /// *you* write that relies on the behavior of this function may break.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    /// # Aborts
    ///
    /// Aborts on OOM.
    pub fn reserve_exact(&mut self, len: u32, additional: u32) {
        self.inner.reserve_exact(len, additional, T::LAYOUT)
    }

    /// The same as `reserve_exact`, but returns on errors instead of panicking or aborting.
    pub fn try_reserve_exact(&mut self, len: u32, additional: u32) -> Result<(), TryReserveError> {
        self.inner.try_reserve_exact(len, additional, T::LAYOUT)
    }

    /// Shrinks the buffer down to the specified capacity. If the given amount
    /// is 0, actually completely deallocates.
    ///
    /// # Panics
    ///
    /// Panics if the given amount is *larger* than the current capacity.
    ///
    /// # Aborts
    ///
    /// Aborts on OOM.
    #[inline]
    pub fn shrink_to_fit(&mut self, cap: u32) {
        self.inner.shrink_to_fit(cap, T::LAYOUT)
    }
}

impl<T> Drop for RawVec<T> {
    /// Frees the memory owned by the `RawVec` *without* trying to drop its contents.
    fn drop(&mut self) {
        // SAFETY: We are in a Drop impl, self.inner will not be used again.
        unsafe { self.inner.deallocate(T::LAYOUT) }
    }
}

impl RawVecInner {
    #[inline]
    const fn new_in(align: u32) -> Self {
        let ptr = unsafe { core::mem::transmute(align) };
        // `cap: 0` means "unallocated". zero-sized types are ignored.
        Self {
            ptr,
            cap: Cap::ZERO,
        }
    }

    #[inline]
    fn with_capacity_in(capacity: u32, elem_layout: Layout) -> Self {
        match Self::try_allocate_in(capacity, elem_layout) {
            Ok(this) => {
                unsafe {
                    // Make it more obvious that a subsquent Vec::reserve(capacity) will not allocate.
                    hint::assert_unchecked(!this.needs_to_grow(0, capacity, elem_layout));
                }
                this
            }
            Err(err) => handle_error(err),
        }
    }

    #[inline]
    fn try_with_capacity_in(capacity: u32, elem_layout: Layout) -> Result<Self, TryReserveError> {
        Self::try_allocate_in(capacity, elem_layout)
    }

    #[inline]
    fn with_capacity_zeroed_in(capacity: u32, elem_layout: Layout) -> Self {
        match Self::try_allocate_in(capacity, elem_layout) {
            Ok(res) => res,
            Err(err) => handle_error(err),
        }
    }

    fn try_allocate_in(capacity: u32, elem_layout: Layout) -> Self {
        let Ok(layout) = layout_array(capacity, elem_layout) else {
            capacity_overflow()
        };

        // Don't allocate here because `Drop` will not deallocate when `capacity` is 0.
        if layout.size() == 0 {
            return Ok(Self::new_in(elem_layout.align()));
        }

        if let Err(err) = alloc_guard(layout.size()) {
            return Err(err);
        }

        let result = alloc.allocate_zeroed(layout);
        let ptr = match result {
            Ok(ptr) => ptr,
            Err(_) => {
                return Err(AllocError {
                    layout,
                    non_exhaustive: (),
                }
                .into())
            }
        };

        // Allocators currently return a `NonNull<[u8]>` whose length
        // matches the size requested. If that ever changes, the capacity
        // here should change to `ptr.len() / mem::size_of::<T>()`.
        Ok(Self {
            ptr: Unique::from(ptr.cast()),
            cap: unsafe { Cap(capacity) },
            alloc,
        })
    }

    #[inline]
    unsafe fn from_raw_parts_in(ptr: *mut u8, cap: Cap) -> Self {
        Self {
            ptr: unsafe { Unique::new_unchecked(ptr) },
            cap,
            alloc,
        }
    }

    #[inline]
    unsafe fn from_nonnull_in(ptr: NonNull<u8>, cap: Cap) -> Self {
        Self {
            ptr: Unique::from(ptr),
            cap,
            alloc,
        }
    }

    #[inline]
    fn ptr<T>(&self) -> *mut T {
        self.non_null::<T>().as_ptr()
    }

    #[inline]
    fn non_null<T>(&self) -> NonNull<T> {
        self.ptr.cast().into()
    }

    #[inline]
    fn capacity(&self, elem_size: u32) -> u32 {
        if elem_size == 0 {
            usize::MAX
        } else {
            self.cap.0
        }
    }

    #[inline]
    fn allocator(&self) -> &A {
        &self.alloc
    }

    #[inline]
    fn current_memory(&self, elem_layout: Layout) -> Option<(NonNull<u8>, Layout)> {
        if elem_layout.size() == 0 || self.cap.0 == 0 {
            None
        } else {
            // We could use Layout::array here which ensures the absence of isize and usize overflows
            // and could hypothetically handle differences between stride and size, but this memory
            // has already been allocated so we know it can't overflow and currently Rust does not
            // support such types. So we can do better by skipping some checks and avoid an unwrap.
            unsafe {
                let alloc_size = elem_layout.size().unchecked_mul(self.cap.0);
                let layout = Layout::from_size_align_unchecked(alloc_size, elem_layout.align());
                Some((self.ptr.into(), layout))
            }
        }
    }

    #[inline]
    fn reserve(&mut self, len: u32, additional: u32, elem_layout: Layout) {
        // Callers expect this function to be very cheap when there is already sufficient capacity.
        // Therefore, we move all the resizing and error-handling logic from grow_amortized and
        // handle_reserve behind a call, while making sure that this function is likely to be
        // inlined as just a comparison and a call if the comparison fails.
        #[cold]
        fn do_reserve_and_handle(
            slf: &mut RawVecInner,
            len: u32,
            additional: u32,
            elem_layout: Layout,
        ) {
            if let Err(err) = slf.grow_amortized(len, additional, elem_layout) {
                handle_error(err);
            }
        }

        if self.needs_to_grow(len, additional, elem_layout) {
            do_reserve_and_handle(self, len, additional, elem_layout);
        }
    }

    #[inline]
    fn grow_one(&mut self, elem_layout: Layout) {
        if let Err(err) = self.grow_amortized(self.cap.0, 1, elem_layout) {
            handle_error(err);
        }
    }

    fn try_reserve(
        &mut self,
        len: u32,
        additional: u32,
        elem_layout: Layout,
    ) -> Result<(), TryReserveError> {
        if self.needs_to_grow(len, additional, elem_layout) {
            self.grow_amortized(len, additional, elem_layout)?;
        }
        unsafe {
            // Inform the optimizer that the reservation has succeeded or wasn't needed
            hint::assert_unchecked(!self.needs_to_grow(len, additional, elem_layout));
        }
        Ok(())
    }

    fn reserve_exact(&mut self, len: u32, additional: u32, elem_layout: Layout) {
        if let Err(err) = self.try_reserve_exact(len, additional, elem_layout) {
            handle_error(err);
        }
    }

    fn try_reserve_exact(
        &mut self,
        len: u32,
        additional: u32,
        elem_layout: Layout,
    ) -> Result<(), TryReserveError> {
        if self.needs_to_grow(len, additional, elem_layout) {
            self.grow_exact(len, additional, elem_layout)?;
        }
        unsafe {
            // Inform the optimizer that the reservation has succeeded or wasn't needed
            hint::assert_unchecked(!self.needs_to_grow(len, additional, elem_layout));
        }
        Ok(())
    }

    #[inline]
    fn shrink_to_fit(&mut self, cap: u32, elem_layout: Layout) {
        if let Err(err) = self.shrink(cap, elem_layout) {
            handle_error(err);
        }
    }

    #[inline]
    fn needs_to_grow(&self, len: u32, additional: u32, elem_layout: Layout) -> bool {
        additional > self.capacity(elem_layout.size()).wrapping_sub(len)
    }

    #[inline]
    unsafe fn set_ptr_and_cap(&mut self, ptr: NonNull<[u8]>, cap: u32) {
        // Allocators currently return a `NonNull<[u8]>` whose length matches
        // the size requested. If that ever changes, the capacity here should
        // change to `ptr.len() / mem::size_of::<T>()`.
        self.ptr = Unique::from(ptr.cast());
        self.cap = unsafe { Cap(cap) };
    }

    fn grow_amortized(&mut self, len: u32, additional: u32, elem_layout: Layout) {
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
        let cap = cmp::max(self.cap.0 * 2, required_cap);
        let cap = cmp::max(min_non_zero_cap(elem_layout.size()), cap);

        let Ok(new_layout) = layout_array(cap, elem_layout) else {
            capacity_overflow()
        };

        let Ok(ptr) = finish_grow(
            new_layout,
            self.current_memory(elem_layout),
            &mut self.alloc,
        ) else {
            capacity_overflow()
        };
        // SAFETY: finish_grow would have resulted in a capacity overflow if we tried to allocate more than `isize::MAX` items

        unsafe { self.set_ptr_and_cap(ptr, cap) };
        Ok(())
    }

    fn grow_exact(&mut self, len: u32, additional: u32, elem_layout: Layout) {
        if elem_layout.size() == 0 {
            // Since we return a capacity of `usize::MAX` when the type size is
            // 0, getting to here necessarily means the `RawVec` is overfull.
            capacity_overflow();
        }

        let Ok(cap) = len.checked_add(additional) else {
            capacity_overflow();
        };
        let Ok(new_layout) = layout_array(cap, elem_layout) else {
            capacity_overflow()
        };

        let Ok(ptr) = finish_grow(
            new_layout,
            self.current_memory(elem_layout),
            &mut self.alloc,
        ) else {
            capacity_overflow()
        };
        // SAFETY: finish_grow would have resulted in a capacity overflow if we tried to allocate more than `isize::MAX` items
        unsafe {
            self.set_ptr_and_cap(ptr, cap);
        }
        Ok(())
    }

    #[inline]
    fn shrink(&mut self, cap: u32, elem_layout: Layout) -> Result<(), TryReserveError> {
        assert!(
            cap <= self.capacity(elem_layout.size()),
            "Tried to shrink to a larger capacity"
        );
        // SAFETY: Just checked this isn't trying to grow
        unsafe { self.shrink_unchecked(cap, elem_layout) }
    }

    /// `shrink`, but without the capacity check.
    ///
    /// This is split out so that `shrink` can inline the check, since it
    /// optimizes out in things like `shrink_to_fit`, without needing to
    /// also inline all this code, as doing that ends up failing the
    /// `vec-shrink-panic` codegen test when `shrink_to_fit` ends up being too
    /// big for LLVM to be willing to inline.
    ///
    /// # Safety
    /// `cap <= self.capacity()`
    unsafe fn shrink_unchecked(
        &mut self,
        cap: u32,
        elem_layout: Layout,
    ) -> Result<(), TryReserveError> {
        let (ptr, layout) = if let Some(mem) = self.current_memory(elem_layout) {
            mem
        } else {
            return Ok(());
        };

        // If shrinking to 0, deallocate the buffer. We don't reach this point
        // for the T::IS_ZST case since current_memory() will have returned
        // None.
        if cap == 0 {
            unsafe { self.alloc.deallocate(ptr, layout) };
            self.ptr = unsafe { Unique::new_unchecked(std::mem::transmute(elem_layout.align())) };
            self.cap = Cap::ZERO;
        } else {
            let ptr = unsafe {
                // Layout cannot overflow here because it would have
                // overflowed earlier when capacity was larger.
                let new_size = elem_layout.size().unchecked_mul(cap);
                let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
                self.alloc
                    .shrink(ptr, layout, new_layout)
                    .unwrap_err(|_| handle_alloc_error(new_layout))
            };
            // SAFETY: if the allocation is valid, then the capacity is too
            unsafe {
                self.set_ptr_and_cap(ptr, cap);
            }
        }
        Ok(())
    }

    /// # Safety
    ///
    /// This function deallocates the owned allocation, but does not update `ptr` or `cap` to
    /// prevent double-free or use-after-free. Essentially, do not do anything with the caller
    /// after this function returns.
    /// Ideally this function would take `self` by move, but it cannot because it exists to be
    /// called from a `Drop` impl.
    unsafe fn deallocate(&mut self, elem_layout: Layout) {
        if let Some((ptr, layout)) = self.current_memory(elem_layout) {
            unsafe {
                self.alloc.deallocate(ptr, layout);
            }
        }
    }
}

#[inline(never)]
fn finish_grow(
    new_layout: Layout,
    current_memory: Option<(NonNull<u8>, Layout)>,
) -> Result<NonNull<[u8]>, TryReserveError> {
    alloc_guard(new_layout.size())?;

    let memory = if let Some((ptr, old_layout)) = current_memory {
        debug_assert_eq!(old_layout.align(), new_layout.align());
        unsafe {
            // The allocator checks for alignment equality
            hint::assert_unchecked(old_layout.align() == new_layout.align());
            realloc(ptr, old_layout, new_layout)
        }
    } else {
        unsafe { alloc(new_layout) }
    };

    memory.map_err(|_| handle_alloc_error(new_layout))
}

// We need to guarantee the following:
// * We don't ever allocate `> isize::MAX` byte-size objects.
// * We don't overflow `usize::MAX` and actually allocate too little.
//
// On 64-bit we just need to check for overflow since trying to allocate
// `> isize::MAX` bytes will surely fail. On 32-bit and 16-bit we need to add
// an extra guard for this in case we're running on a platform which can use
// all 4GB in user-space, e.g., PAE or x32.
#[inline]
fn alloc_guard(alloc_size: u32) -> Result<(), ()> {
    if usize::BITS < 64 && alloc_size > isize::MAX as usize {
        Err(())
    } else {
        Ok(())
    }
}

#[inline]
fn layout_array(cap: u32, elem_layout: Layout) -> Result<Layout, ()> {
    elem_layout
        .repeat(cap)
        .map(|(layout, _pad)| layout)
        .map_err(|_| ())
}
