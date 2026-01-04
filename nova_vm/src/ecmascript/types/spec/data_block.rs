// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [6.2.9 Data Blocks](https://tc39.es/ecma262/#sec-data-blocks)

#[cfg(feature = "shared-array-buffer")]
use core::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "shared-array-buffer")]
use std::hint::assert_unchecked;

use core::{
    f32, f64,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::{NonNull, read_unaligned, write_unaligned},
};
use std::{
    alloc::{Layout, alloc_zeroed, dealloc, handle_alloc_error, realloc},
    ops::{BitAnd, BitOr, BitXor},
};

use ecmascript_atomics::RacyStorage;
#[cfg(feature = "shared-array-buffer")]
use ecmascript_atomics::{Ordering as ECMAScriptOrdering, RacyPtr, RacySlice};
use num_bigint::Sign;

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int64_big_int, to_big_uint64_big_int, to_int8_number, to_int16_number,
            to_int32_number, to_uint8_clamp_number, to_uint8_number, to_uint16_number,
            to_uint32_number,
        },
        execution::{Agent, JsResult, agent::ExceptionType},
        types::{BigInt, Number, Numeric, Value},
    },
    engine::context::{NoGcScope, trivially_bindable},
    heap::ArenaAccess,
};

#[cfg(feature = "array-buffer")]
use crate::ecmascript::execution::ProtoIntrinsics;

/// # [Data Block](https://tc39.es/ecma262/#sec-data-blocks)
///
/// The Data Block specification type is used to describe a distinct and
/// mutable sequence of byte-sized (8 bit) numeric values. A byte value is an
/// integer in the inclusive interval from 0 to 255. A Data Block value is
/// created with a fixed number of bytes that each have the initial value 0.
///
/// The `ptr` points to a continuous buffer of bytes, the length of which is
/// determined by the capacity. The pointer can be None if the block is
/// detached.
pub(crate) struct DataBlock {
    ptr: Option<NonNull<u8>>,
    byte_length: usize,
}
trivially_bindable!(DataBlock);

impl core::fmt::Debug for DataBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let slice = if let Some(ptr) = self.ptr {
            // SAFETY: ptr points to a valid allocation of byte_length bytes.
            unsafe { core::slice::from_raw_parts(ptr.as_ptr(), self.byte_length) }
        } else {
            &[]
        };
        slice.fmt(f)
    }
}

impl Drop for DataBlock {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            if self.byte_length == 0 {
                // dangling data block; don't dealloc.
                return;
            }
            let layout = Layout::from_size_align(self.byte_length, 8).unwrap();
            unsafe { dealloc(ptr.as_ptr(), layout) }
        }
    }
}

impl Deref for DataBlock {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        if let Some(ptr) = self.ptr {
            // SAFETY: DataBlock has a non-null, pointer. We guarantee it
            // points to a valid allocation of byte_length initialized bytes
            // (note, length can be 0 in which case pointer is dangling).
            unsafe { std::slice::from_raw_parts(ptr.as_ptr(), self.byte_length) }
        } else {
            &[]
        }
    }
}

impl DerefMut for DataBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(mut ptr) = self.ptr {
            // SAFETY: DataBlock has a non-null, pointer. We guarantee it
            // points to a valid allocation of byte_length initialized bytes
            // (note, length can be 0 in which case pointer is dangling).
            unsafe { std::slice::from_raw_parts_mut(ptr.as_mut(), self.byte_length) }
        } else {
            &mut []
        }
    }
}

impl DataBlock {
    /// A detached DataBlock.
    pub(crate) const DETACHED_DATA_BLOCK: DataBlock = DataBlock {
        ptr: None,
        byte_length: 0,
    };

    /// An empty DataBlock.
    const EMPTY_DATA_BLOCK: DataBlock = DataBlock {
        ptr: Some(NonNull::<usize>::dangling().cast::<u8>()),
        byte_length: 0,
    };

    #[inline(always)]
    pub(crate) fn is_detached(&self) -> bool {
        self.ptr.is_none()
    }

    /// Allocate a new DataBlock with the given size.
    fn new(len: usize) -> Option<Self> {
        if len == 0 {
            Some(Self::EMPTY_DATA_BLOCK)
        } else {
            let Ok(layout) = Layout::from_size_align(len, 8) else {
                return None;
            };
            // SAFETY: Size of allocation is non-zero.
            let ptr = unsafe { alloc_zeroed(layout) };
            let ptr = NonNull::new(ptr)?;
            debug_assert_eq!(ptr.align_offset(8), 0);
            Some(Self {
                ptr: Some(ptr),
                byte_length: len,
            })
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.byte_length
    }

    fn as_ptr(&self, byte_offset: usize) -> Option<*const u8> {
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { data.as_ptr().add(byte_offset) as *const _ }
            })
        }
    }

    fn as_mut_ptr(&mut self, byte_offset: usize) -> Option<*mut u8> {
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { data.as_ptr().add(byte_offset) }
            })
        }
    }

    #[allow(unused)]
    pub(crate) fn get<T: Viewable>(&self, offset: usize) -> Option<T> {
        let size = core::mem::size_of::<T>();
        let byte_offset = offset * size;
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { read_unaligned(data.as_ptr().add(offset).cast()) }
            })
        }
    }

    /// Read a T from the buffer at `byte_offset`.
    ///
    /// # Safety
    ///
    /// The buffer must have enough room to read a T at `byte_offset` and must
    /// not be detached. The offset must be aligned.
    #[inline(always)]
    pub(crate) unsafe fn read_aligned<T: Viewable>(&self, byte_offset: usize) -> T {
        // SAFETY: The data is properly initialized, and the T being read is
        // checked to be fully within the length of the data allocation.
        unsafe {
            core::ptr::read(
                self.ptr
                    .unwrap_unchecked()
                    .as_ptr()
                    .byte_add(byte_offset)
                    .cast(),
            )
        }
    }

    /// Read a T from the buffer at `byte_offset`.
    ///
    /// # Safety
    ///
    /// The buffer must have enough room to read a T at `byte_offset` and must
    /// not be detached.
    #[inline(always)]
    pub(crate) unsafe fn read_unaligned<T: Viewable>(&self, byte_offset: usize) -> T {
        // SAFETY: The data is properly initialized, and the T being read is
        // checked to be fully within the length of the data allocation.
        unsafe {
            core::ptr::read_unaligned(
                self.ptr
                    .unwrap_unchecked()
                    .as_ptr()
                    .byte_add(byte_offset)
                    .cast(),
            )
        }
    }

    #[allow(unused)]
    pub(crate) fn set<T: Viewable>(&mut self, offset: usize, value: T) {
        let size = core::mem::size_of::<T>();
        if let Some(data) = self.ptr {
            // Note: We have to check offset + 1 to ensure that the write does
            // not reach data beyond the end of the DataBlock allocation.
            let end_byte_offset = (offset + 1) * size;
            if end_byte_offset <= self.byte_length {
                // SAFETY: The data is properly initialized, and the T being written is
                // checked to be fully within the length of the data allocation.
                unsafe { write_unaligned(data.as_ptr().add(offset).cast(), value) }
            }
        }
    }

    pub(crate) fn set_offset_by_byte<T: Viewable>(&mut self, byte_offset: usize, value: T) {
        let size = core::mem::size_of::<T>();
        if let Some(data) = self.ptr {
            // Note: We have to check offset + 1 to ensure that the write does
            // not reach data beyond the end of the DataBlock allocation.
            let end_byte_offset = byte_offset + size;
            if end_byte_offset <= self.byte_length {
                // SAFETY: The data is properly initialized, and the T being written is
                // checked to be fully within the length of the data allocation.
                unsafe { write_unaligned(data.as_ptr().byte_add(byte_offset).cast(), value) }
            }
        }
    }

    #[allow(unused)]
    pub(crate) fn set_from<T: Viewable>(
        &mut self,
        dst_offset: usize,
        src: &DataBlock,
        src_offset: usize,
        count: usize,
    ) {
        let size = core::mem::size_of::<T>();
        let byte_length = count * size;
        if byte_length == 0 {
            return;
        }
        let dst_byte_offset = dst_offset * size;
        let src_byte_offset = src_offset * size;
        debug_assert!(dst_byte_offset + byte_length <= self.byte_length);
        debug_assert!(src_byte_offset + byte_length <= src.byte_length);
        let src_ptr = src.as_ptr(src_byte_offset);
        let dst_ptr = self.as_mut_ptr(dst_byte_offset);
        if let (Some(src), Some(dst)) = (src_ptr, dst_ptr) {
            // SAFETY: Source buffer length is valid, destination buffer
            // is likewise at least equal in length to source, and both
            // are properly aligned for bytes.
            unsafe { dst.copy_from_nonoverlapping(src, byte_length) }
        }
    }

    #[allow(unused)]
    pub(crate) fn copy_within<T: Viewable>(
        &mut self,
        dst_offset: usize,
        src_offset: usize,
        count: usize,
    ) {
        let size = core::mem::size_of::<T>();
        let byte_length = count * size;
        if byte_length == 0 {
            return;
        }
        let dst_byte_offset = dst_offset * size;
        let src_byte_offset = src_offset * size;
        debug_assert!(dst_byte_offset + byte_length <= self.byte_length);
        debug_assert!(src_byte_offset + byte_length <= self.byte_length);
        if let Some(ptr) = self.as_mut_ptr(0) {
            // SAFETY: Buffer is valid for reads and writes of u8 for the whole length.
            let slice = unsafe { core::slice::from_raw_parts_mut(ptr, self.byte_length) };
            slice.copy_within(
                src_byte_offset..(src_byte_offset + byte_length),
                dst_byte_offset,
            );
        }
    }

    pub(crate) fn realloc(&mut self, new_byte_length: usize) {
        // Max byte length should be within safe integer length.
        debug_assert!(new_byte_length < 2usize.pow(53));
        let ptr = if let Some(ptr) = self.ptr {
            if new_byte_length == 0 {
                // When resizing to zero, we can just reassign self to an empty
                // data block; that drops the previous block which deallocs.
                *self = Self::EMPTY_DATA_BLOCK;
                return;
            } else {
                // SAFETY: `ptr` can currently only come from GlobalAllocator, it was
                // allocated with `Layout::from_size_align(self.byte_length, 8)`, new
                // size is non-zero, and cannot overflow isize (on a 64-bit machine).
                if self.byte_length > 0 {
                    let layout = Layout::from_size_align(self.byte_length, 8).unwrap();
                    unsafe { realloc(ptr.as_ptr(), layout, new_byte_length) }
                } else {
                    let layout = Layout::from_size_align(new_byte_length, 8).unwrap();
                    unsafe { alloc_zeroed(layout) }
                }
            }
        } else {
            // Detached.
            return;
        };
        let Some(ptr) = NonNull::new(ptr) else {
            let layout = Layout::from_size_align(new_byte_length, 8).unwrap();
            handle_alloc_error(layout);
        };
        self.ptr = Some(ptr);
        if new_byte_length > self.byte_length {
            // Need to zero out the new data.
            // SAFETY: The new pointer does point to valid data which is
            // big enough.
            let new_data_ptr = unsafe { ptr.add(self.byte_length) };
            // SAFETY: The new pointer does point to valid, big enough
            // allocation which contains uninitialized bytes. No one else
            // can hold a reference to it currently.
            let data_slice = unsafe {
                core::slice::from_raw_parts_mut(
                    new_data_ptr.as_ptr().cast::<MaybeUninit<u8>>(),
                    new_byte_length - self.byte_length,
                )
            };
            data_slice.fill(MaybeUninit::new(0));
        }
        self.byte_length = new_byte_length;
    }
}

/// Maximum byte length of a SharedArrayBuffer.
///
/// If the top bit is set, then this is a growable SharedDataBlock, and the
/// byte length of the SharedArrayBuffer is stored in the buffer allocation.
/// Note that growable SharedDataBlocks can still be dangling; in this case
/// their maximum byte length has only the top bit set, ie. they have a
/// maximum byte length value of zero.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "shared-array-buffer")]
struct SharedDataBlockMaxByteLength(usize);

#[cfg(feature = "shared-array-buffer")]
impl SharedDataBlockMaxByteLength {
    /// Create a new max byte length value for a possibly growable SharedDataBlock.
    #[inline(always)]
    fn new(size: usize, growable: bool) -> Self {
        if growable {
            if cfg!(target_pointer_width = "64") {
                Self(size | 0x8000_0000_0000_0000)
            } else if cfg!(target_pointer_width = "32") {
                Self(size | 0x8000_0000)
            } else {
                panic!()
            }
        } else {
            Self(size)
        }
    }

    /// Get the maximum byte length value of a SharedDataBlock.
    ///
    /// Note that the maximum byte length of a growable SharedArrayBuffer or
    /// the byte length of an static SharedArrayBuffer.
    #[inline(always)]
    fn max_byte_length(self) -> usize {
        if cfg!(target_pointer_width = "64") {
            self.0 & 0x7FFF_FFFF_FFFF_FFFF
        } else if cfg!(target_pointer_width = "32") {
            self.0 & 0x7FFF_FFFF
        } else {
            panic!()
        }
    }

    /// Returns true if the SharedDataBlock has a 0 maximum byte length.
    #[inline(always)]
    fn is_dangling(self) -> bool {
        self.max_byte_length() == 0
    }

    /// Returns true if the SharedDataBlock is growable.
    #[inline(always)]
    fn is_growable(self) -> bool {
        self.0.leading_ones() > 0
    }
}

/// # [Shared Data Block](https://tc39.es/ecma262/#sec-data-blocks)
///
/// The Shared Data Block specification type is used to describe a distinct and
/// mutable sequence of byte-sized (8 bit) atomic numeric values. A byte value
/// is an integer in the inclusive interval from 0 to 255. A Shared Data Block
/// value is created with a fixed number of bytes that each have the initial
/// value 0.
///
/// The `ptr` points to a continuous buffer of bytes, the length of which is
/// determined by the capacity. Before the buffer of bytes, a usize is
/// allocated that is used for reference counting.
///
/// ## Buffer memory layout
///
/// For statically sized SharedDataBlocks, the buffer memory layout is the
/// following:
///
/// ```rust,ignore
/// #[repr(C)]
/// struct StaticSharedDataBuffer<const N: usize> {
///   rc: AtomicUsize,
///   bytes: [AtomicU8; N],
/// }
/// ```
///
/// and for growable SharedDataBlocks, the layout is:
/// ```rust,ignore
/// #[repr(C)]
/// struct GrowableSharedDataBuffer<const N: usize> {
///   byte_length: AtomicUsize,
///   rc: AtomicUsize,
///   bytes: [AtomicU8; N],
/// }
/// ```
///
/// The `ptr` field points to the start of the `bytes`
///
/// Note that the "viewed" byte length of the buffer is defined inside the
/// buffer when the SharedDataBlock is growable.
#[must_use]
#[repr(C)]
#[derive(PartialEq, Eq)]
#[cfg(feature = "shared-array-buffer")]
pub struct SharedDataBlock {
    ptr: RacyPtr<u8>,
    max_byte_length: SharedDataBlockMaxByteLength,
}

// SAFETY: Atomic RC.
#[cfg(feature = "shared-array-buffer")]
unsafe impl Send for SharedDataBlock {}
// SAFETY: Atomic RC.
#[cfg(feature = "shared-array-buffer")]
unsafe impl Sync for SharedDataBlock {}

#[cfg(feature = "shared-array-buffer")]
impl core::default::Default for SharedDataBlock {
    fn default() -> Self {
        Self::DANGLING_STATIC_SHARED_DATA_BLOCK
    }
}

#[cfg(feature = "shared-array-buffer")]
impl Clone for SharedDataBlock {
    fn clone(&self) -> Self {
        if !self.is_dangling() {
            // Dangling blocks do not increment any RC.
            // SAFETY: not dangling.
            let rc = unsafe { self.get_rc() };
            rc.fetch_add(1, Ordering::Relaxed);
        }
        Self {
            ptr: self.ptr,
            max_byte_length: self.max_byte_length,
        }
    }
}

#[cfg(feature = "shared-array-buffer")]
impl Drop for SharedDataBlock {
    fn drop(&mut self) {
        use ecmascript_atomics::RacyMemory;

        if self.is_dangling() {
            // dangling shared data block; don't dealloc.
            return;
        }
        let growable = self.is_growable();
        // SAFETY: SharedDataBlock guarantees we have a AtomicUsize allocated
        // before the bytes.
        let rc_ptr = unsafe { self.ptr.as_ptr().cast::<AtomicUsize>().sub(1) };
        {
            // SAFETY: the RC is definitely still allocated, as we haven't
            // subtracted ourselves from it yet.
            let rc = unsafe { rc_ptr.as_ref() };
            // Make a guess: we're the only holder.
            let mut value = 1;
            let mut new_value = 0;
            // loop over the exchange; if we fail to update, take the previous
            // value and try to subtract one.
            while let Err(previous_value) =
                rc.compare_exchange_weak(value, new_value, Ordering::Relaxed, Ordering::Relaxed)
            {
                value = previous_value;
                let Some(value) = value.checked_sub(1) else {
                    // someone else dropped the value.
                    return;
                };
                new_value = value;
            }
            if new_value > 0 {
                // We were not the last item; do not deallocate the buffer.
                return;
            }
        }
        let max_byte_length = self.max_byte_length();
        // SAFETY: if we're here then we're the last holder of the data block.
        let (size, base_ptr) = if growable {
            // This is a growable SharedDataBlock that we're working with here.

            // SAFETY: layout guaranteed by type
            unsafe {
                (
                    max_byte_length
                        .unchecked_add(core::mem::size_of::<(AtomicUsize, AtomicUsize)>()),
                    rc_ptr.sub(1),
                )
            }
        } else {
            unsafe {
                (
                    max_byte_length.unchecked_add(core::mem::size_of::<AtomicUsize>()),
                    rc_ptr,
                )
            }
        };
        let memory = RacyMemory::from_raw_parts(self.ptr, max_byte_length);
        // SAFETY: As per the CAS loop on the reference count, we are the only
        // referrer to the racy memory. We can thus deallocate the ECMAScript
        // memory; this effectively grows our Rust memory from being just the
        // RC and possible byte length value, into also containing the byte
        // data.
        let _ = unsafe { memory.exit() };
        // SAFETY: layout guaranteed by type.
        let layout = unsafe { Layout::from_size_align(size, 8).unwrap_unchecked() };
        unsafe { dealloc(base_ptr.cast::<u8>().as_ptr(), layout) }
    }
}

#[cfg(feature = "shared-array-buffer")]
impl SharedDataBlock {
    const DANGLING_STATIC_SHARED_DATA_BLOCK: Self = Self {
        ptr: RacyPtr::dangling(),
        max_byte_length: SharedDataBlockMaxByteLength(0),
    };

    const DANGLING_GROWABLE_SHARED_DATA_BLOCK: Self = Self {
        ptr: RacyPtr::dangling(),
        max_byte_length: SharedDataBlockMaxByteLength(1usize.rotate_right(1)),
    };

    /// Allocate a new SharedDataBlock.
    ///
    /// ## Safety
    ///
    /// `byte_length` is less or equal to `max_byte_length` if defined.
    unsafe fn new(byte_length: usize, max_byte_length: Option<usize>) -> Option<Self> {
        let size = max_byte_length.unwrap_or(byte_length);
        unsafe {
            assert_unchecked(byte_length <= size);
        }
        let growable = max_byte_length.is_some();
        if size == 0 {
            if growable {
                Some(Self::DANGLING_GROWABLE_SHARED_DATA_BLOCK)
            } else {
                Some(Self::DANGLING_STATIC_SHARED_DATA_BLOCK)
            }
        } else {
            // Note: if we start supporting 32-bit environments which do not
            // have 64-bit atomics, then we need to allocate a lock word to the
            // beginning as well.

            use ecmascript_atomics::RacyMemory;
            let alloc_size = if growable {
                // Growable SharedArrayBuffer
                size.checked_add(core::mem::size_of::<(AtomicUsize, AtomicUsize)>())?
            } else {
                // Static SharedArrayBuffer
                size.checked_add(core::mem::size_of::<AtomicUsize>())?
            };
            let Ok(layout) = Layout::from_size_align(alloc_size, 8) else {
                return None;
            };
            // SAFETY: Size of allocation is non-zero.
            let base_ptr = unsafe { alloc_zeroed(layout) };
            let base_ptr = NonNull::new(base_ptr)?.cast::<usize>();
            unsafe { assert_unchecked(base_ptr.is_aligned()) };
            let rc_ptr = if growable {
                // Growable SharedArrayBuffer; write the byte length here.
                // SAFETY: properly allocated, everything is fine.
                unsafe { base_ptr.write(byte_length) };
                // SAFETY: allocation size is
                // (AtomicUsize, AtomicUsize, [AtomicU8; max_byte_length])
                unsafe { base_ptr.add(1) }
            } else {
                base_ptr
            };
            {
                // SAFETY: we're the only owner of this data.
                unsafe { rc_ptr.write(1) };
            }
            // SAFETY: the pointer is len + usize
            let ptr = unsafe { rc_ptr.add(1) };
            // SAFETY: ptr does point to size bytes of readable and writable
            // Rust memory. After this call, that memory is deallocated and we
            // receive a new RacyMemory in its stead. Reads and writes through
            // it are undefined behaviour. Note though that we still have the
            // RC and possible length values before the pointer; those are in
            // normal Rust memory.
            let ptr = unsafe { RacyMemory::<u8>::enter(ptr.cast(), size) };
            Some(Self {
                ptr: ptr.as_slice().into_raw_parts().0,
                max_byte_length: SharedDataBlockMaxByteLength::new(size, growable),
            })
        }
    }

    /// Get a racy memory slice from the SharedDataBlock.
    pub(crate) fn as_racy_slice(&self) -> RacySlice<'_, u8> {
        // SAFETY: Type guarantees that ptr is backed by at least byte_length
        // racy bytes.
        unsafe {
            RacySlice::from_raw_parts(self.ptr, self.byte_length(ECMAScriptOrdering::Unordered))
        }
    }

    /// Get a reference to the atomic reference counter.
    ///
    /// ## Safety
    ///
    /// Must not be a dangling SharedDataBlock.
    unsafe fn get_rc(&self) -> &AtomicUsize {
        // SAFETY: type guarantees layout
        unsafe { self.ptr.as_ptr().cast::<AtomicUsize>().sub(1).as_ref() }
    }

    /// Get a reference to the atomic byte length.
    ///
    /// ## Safety
    ///
    /// Must be a growable, non-dangling SharedDataBlock.
    unsafe fn get_byte_length(&self) -> &AtomicUsize {
        // SAFETY: caller guarantees growable; type guarantees layout.
        unsafe { self.ptr.as_ptr().cast::<AtomicUsize>().sub(2).as_ref() }
    }

    /// Returns the byte length of the SharedArrayBuffer.
    ///
    /// Note that if this is a growable SharedArrayBuffer, then the byte length
    /// can be grown from other threads and reading it is a sequentially
    /// consistent atomic operation.
    #[inline(always)]
    pub(crate) fn byte_length(&self, order: ECMAScriptOrdering) -> usize {
        if self.is_dangling() {
            return 0;
        }
        if self.is_growable() {
            // Need to read the byte length atomically.
            // SAFETY: This is non-dangling growable SharedDataBlock, so the
            // pointer points to the racy memory of
            // `(AtomicUsize, AtomicUsize, ...racy memory...)`
            let byte_length = unsafe { self.get_byte_length() };
            byte_length.load(match order {
                ECMAScriptOrdering::Unordered => Ordering::Relaxed,
                ECMAScriptOrdering::SeqCst => Ordering::SeqCst,
            })
        } else {
            self.max_byte_length()
        }
    }

    /// Get the maximum byte length of the SharedDataBlock.
    #[inline(always)]
    pub(crate) fn max_byte_length(&self) -> usize {
        self.max_byte_length.max_byte_length()
    }

    /// Return true if the SharedDataBlock is a dangling (has maximum byte
    /// length equal to zero).
    #[inline(always)]
    pub(crate) fn is_dangling(&self) -> bool {
        self.max_byte_length.is_dangling()
    }

    /// Returns true if the SharedDataBlock is growable.
    #[inline(always)]
    pub(crate) fn is_growable(&self) -> bool {
        self.max_byte_length.is_growable()
    }

    /// Read a value at the given aligned offset and with the given ordering.
    ///
    /// Returns `None` if the offset is not correctly aligned or the index is
    /// out of bounds.
    ///
    /// # Soundness
    ///
    /// There is no read in the Rust world: we use the racy atomic operations
    /// of ecmascript_atomics, which tries its best to make sure we don't leak
    /// the status of the memory to Rust. As such, this should be pretty okay.
    #[inline(always)]
    pub(crate) fn load<T: Viewable>(
        &self,
        byte_offset: usize,
        order: ECMAScriptOrdering,
    ) -> Option<T> {
        let slice = self.as_racy_slice().slice_from(byte_offset);
        if const { size_of::<T>() == size_of::<u8>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u8>, Option<T>>(
                    &slice.get(0).map(|s| s.load(order)),
                )
            }
        } else if const { size_of::<T>() == size_of::<u16>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u16>, Option<T>>(
                    &slice.as_aligned::<u16>().map(|t| t.load(order)),
                )
            }
        } else if const { size_of::<T>() == size_of::<u32>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u32>, Option<T>>(
                    &slice.as_aligned::<u32>().map(|t| t.load(order)),
                )
            }
        } else if const { size_of::<T>() == size_of::<u64>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u64>, Option<T>>(
                    &slice.as_aligned::<u64>().map(|t| t.load(order)),
                )
            }
        } else {
            unreachable!("Unexpected load type")
        }
    }

    /// Read a value at the given possibly unaligned offset with no
    /// synchronisation. This read may tear.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Soundness
    ///
    /// There is no read in the Rust world: we use the racy atomic operations
    /// of ecmascript_atomics, which tries its best to make sure we don't leak
    /// the status of the memory to Rust. As such, this should be pretty okay.
    #[inline(always)]
    pub(crate) fn load_unaligned<T: Viewable>(&self, byte_offset: usize) -> Option<T> {
        let slice = self.as_racy_slice().slice_from(byte_offset);
        if const { size_of::<T>() == size_of::<u8>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u8>, Option<T>>(&slice.load_unaligned::<u8>())
            }
        } else if const { size_of::<T>() == size_of::<u16>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u16>, Option<T>>(&slice.load_unaligned::<u16>())
            }
        } else if const { size_of::<T>() == size_of::<u32>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u32>, Option<T>>(&slice.load_unaligned::<u32>())
            }
        } else if const { size_of::<T>() == size_of::<u64>() } {
            // SAFETY: Type checked to match.
            unsafe {
                core::mem::transmute_copy::<Option<u64>, Option<T>>(&slice.load_unaligned::<u64>())
            }
        } else {
            unreachable!("Unexpected load_unaligned type")
        }
    }

    /// Write a value at the given offset and the given ordering.
    ///
    /// Returns `None` if the offset is not correctly aligned or the index is
    /// out of bounds.
    ///
    /// # Soundness
    ///
    /// There is no write in the Rust world: this should be pretty okay.
    pub(crate) fn store<T: Viewable>(
        &self,
        byte_offset: usize,
        val: T,
        order: ECMAScriptOrdering,
    ) -> Option<()> {
        let slice = self.as_racy_slice().slice_from(byte_offset);
        if const { size_of::<T>() == size_of::<u8>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u8>(&val) };
            slice.as_aligned::<u8>().map(|t| t.store(val, order))
        } else if const { size_of::<T>() == size_of::<u16>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u16>(&val) };
            slice.as_aligned::<u16>().map(|t| t.store(val, order))
        } else if const { size_of::<T>() == size_of::<u32>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u32>(&val) };
            slice.as_aligned::<u32>().map(|t| t.store(val, order))
        } else if const { size_of::<T>() == size_of::<u64>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u64>(&val) };
            slice.as_aligned::<u64>().map(|t| t.store(val, order))
        } else {
            unreachable!("Unexpected read type {:?}", core::any::type_name::<T>())
        }
    }

    /// Write a value at the given possibly unaligned offset with no
    /// synchronisation. This write may tear.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Soundness
    ///
    /// There is no write in the Rust world: this should be pretty okay.
    pub(crate) fn store_unaligned<T: Viewable>(&self, byte_offset: usize, val: T) -> Option<()> {
        let slice = self.as_racy_slice().slice_from(byte_offset);
        if const { size_of::<T>() == size_of::<u8>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u8>(&val) };
            slice.store_unaligned::<u8>(val)
        } else if const { size_of::<T>() == size_of::<u16>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u16>(&val) };
            slice.store_unaligned::<u16>(val)
        } else if const { size_of::<T>() == size_of::<u32>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u32>(&val) };
            slice.store_unaligned::<u32>(val)
        } else if const { size_of::<T>() == size_of::<u64>() } {
            // SAFETY: Type checked to match.
            let val = unsafe { core::mem::transmute_copy::<T, u64>(&val) };
            slice.store_unaligned::<u64>(val)
        } else {
            unreachable!("Unexpected read type")
        }
    }

    /// Attempt to grow the SharedDataBlock. Returns false if `new_byte_length`
    /// is or becomes less than the current byte length.
    ///
    /// Note that this is a synchronising compare-and-exchange loop.
    ///
    /// ## Safety
    ///
    /// Must be a growable, non-dangling SharedDataBlock with a maximum byte
    /// length greater or equal to `new_byte_length`.
    pub(crate) unsafe fn grow(&self, new_byte_length: usize) -> bool {
        // SAFETY: precondition.
        let byte_length = unsafe {
            assert_unchecked(self.is_growable());
            assert_unchecked(!self.is_dangling());
            assert_unchecked(new_byte_length <= self.max_byte_length());
            self.get_byte_length()
        };
        // 9. Let currentByteLengthRawBytes be
        //    GetRawBytesFromSharedBlock(byteLengthBlock, 0, biguint64, true, seq-cst).
        let mut current_byte_length = byte_length.load(Ordering::SeqCst);
        loop {
            // c. If newByteLength = currentByteLength,
            if new_byte_length == current_byte_length {
                // return undefined.
                return true;
            }
            // d. If newByteLength < currentByteLength or
            //    newByteLength > O.[[ArrayBufferMaxByteLength]],
            //    throw a RangeError exception.
            if new_byte_length < current_byte_length {
                return false;
            }
            // h. Let readByteLengthRawBytes be
            //    AtomicCompareExchangeInSharedBlock(byteLengthBlock, 0, 8,
            //    currentByteLengthRawBytes, newByteLengthRawBytes).
            let Err(read_byte_length) = byte_length.compare_exchange(
                current_byte_length,
                new_byte_length,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) else {
                // i. If ByteListEqual(readByteLengthRawBytes,
                //    currentByteLengthRawBytes) is true, return undefined.
                return true;
            };
            // j. Set currentByteLengthRawBytes to readByteLengthRawBytes.
            current_byte_length = read_byte_length
        }
    }
}

const DATA_BLOCK_SIZE_LIMIT: u64 = u64::pow(2, 53) - 1;

/// ### [6.2.9.1 CreateByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createbytedatablock)
///
/// The abstract operation CreateByteDataBlock takes argument size (a
/// non-negative integer) and returns either a normal completion containing
/// a Data Block or a throw completion.
pub(crate) fn create_byte_data_block<'a>(
    agent: &mut Agent,
    size: u64,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, DataBlock> {
    // 1. If size > 2**53 - 1, throw a RangeError exception.
    if let Some(db) = usize::try_from(size)
        .ok()
        .and_then(|size| {
            if size as u64 > DATA_BLOCK_SIZE_LIMIT {
                None
            } else {
                Some(size)
            }
        })
        .and_then(DataBlock::new)
    {
        // 2. Let db be a new Data Block value consisting of size bytes.
        // 3. Set all of the bytes of db to 0.
        // 4. Return db.
        Ok(db)
    } else {
        // 2. cont: If it is impossible to create such a Data Block, throw a
        //    RangeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Failed to allocate ArrayBuffer",
            gc,
        ))
    }
}

/// ### [6.2.9.2 CreateSharedByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createsharedbytedatablock)
///
/// The abstract operation CreateSharedByteDataBlock takes argument size (a
/// non-negative integer) and returns either a normal completion containing
/// a Shared Data Block or a throw completion.
///
/// ## Safety
///
/// `byte_length` must be less or equal than `max_byte_length` if it has been
/// defined.
#[cfg(feature = "shared-array-buffer")]
pub(crate) unsafe fn create_shared_byte_data_block<'a>(
    agent: &mut Agent,
    byte_length: u64,
    max_byte_length: Option<u64>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, SharedDataBlock> {
    let size = max_byte_length.unwrap_or(byte_length);
    unsafe {
        assert_unchecked(byte_length <= size);
    }
    // 1. Let db be a new Shared Data Block value consisting of size bytes. If
    //    it is impossible to create such a Shared Data Block, throw a
    //    RangeError exception.
    if let Some(db) = usize::try_from(size)
        .ok()
        .and_then(|size| {
            if size as u64 > DATA_BLOCK_SIZE_LIMIT {
                None
            } else {
                Some(size)
            }
        })
        .and_then(|_| {
            // SAFETY: function precondition
            unsafe {
                SharedDataBlock::new(byte_length as usize, max_byte_length.map(|s| s as usize))
            }
        })
    {
        // 2. Let execution be the [[CandidateExecution]] field of the
        //    surrounding agent's Agent Record.
        // 3. Let eventsRecord be the Agent Events Record of
        //    execution.[[EventsRecords]] whose [[AgentSignifier]] is
        //    AgentSignifier().
        // 4. Let zero be « 0 ».
        // 5. For each index i of db, do
        // a. Append WriteSharedMemory { [[Order]]: INIT, [[NoTear]]: true,
        //    [[Block]]: db, [[ByteIndex]]: i, [[ElementSize]]: 1,
        //    [[Payload]]: zero } to eventsRecord.[[EventList]].
        // 6. Return db.
        Ok(db)
    } else {
        // 2. cont: If it is impossible to create such a Data Block, throw a
        //    RangeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Failed to allocate ArrayBuffer",
            gc,
        ))
    }
}

/// ### [6.2.9.3 CopyDataBlockBytes ( toBlock, toIndex, fromBlock, fromIndex, count )](https://tc39.es/ecma262/#sec-copydatablockbytes)
///
/// The abstract operation CopyDataBlockBytes takes arguments toBlock (a
/// Data Block or a Shared Data Block), toIndex (a non-negative integer),
/// fromBlock (a Data Block or a Shared Data Block), fromIndex (a
/// non-negative integer), and count (a non-negative integer) and returns
/// UNUSED.
pub(crate) fn copy_data_block_bytes(
    to_block: &mut DataBlock,
    to_index: usize,
    from_block: &DataBlock,
    from_index: usize,
    count: usize,
) {
    // 1. Assert: fromBlock and toBlock are distinct values.
    assert!(
        to_block.ptr.is_none()
            || from_block.ptr.is_none()
            || unsafe {
                to_block.ptr.unwrap().as_ptr().add(to_block.len())
                    <= from_block.ptr.unwrap().as_ptr()
                    || from_block.ptr.unwrap().as_ptr().add(from_block.len())
                        <= to_block.ptr.unwrap().as_ptr()
            }
    );
    // 2. Let fromSize be the number of bytes in fromBlock.
    let from_size = from_block.byte_length;
    // 3. Assert: fromIndex + count ≤ fromSize.
    assert!(from_index + count <= from_size);
    // 4. Let toSize be the number of bytes in toBlock.
    let to_size = to_block.byte_length;
    // 5. Assert: toIndex + count ≤ toSize.
    assert!(to_index + count <= to_size);
    // 6. Repeat, while count > 0,
    // i. Assert: toBlock is not a Shared Data Block.
    // ii. Set toBlock[toIndex] to fromBlock[fromIndex].
    // c. Set toIndex to toIndex + 1.
    // d. Set fromIndex to fromIndex + 1.
    // e. Set count to count - 1.
    let to_ptr = if let Some(ptr) = to_block.as_mut_ptr(to_index) {
        ptr
    } else {
        return;
    };
    let from_ptr = if let Some(ptr) = from_block.as_ptr(from_index) {
        ptr
    } else {
        return;
    };
    // SAFETY: Pointers have been checked to not overlap.
    unsafe { to_ptr.copy_from_nonoverlapping(from_ptr, count) };
    // 7. Return UNUSED.
}

/// ### [6.2.9.3 CopyDataBlockBytes ( toBlock, toIndex, fromBlock, fromIndex, count )](https://tc39.es/ecma262/#sec-copydatablockbytes)
///
/// The abstract operation CopyDataBlockBytes takes arguments toBlock (a
/// Data Block or a Shared Data Block), toIndex (a non-negative integer),
/// fromBlock (a Data Block or a Shared Data Block), fromIndex (a
/// non-negative integer), and count (a non-negative integer) and returns
/// UNUSED.
#[cfg(feature = "shared-array-buffer")]
pub(crate) fn copy_shared_data_block_bytes(
    to_block: &SharedDataBlock,
    to_index: usize,
    from_block: &SharedDataBlock,
    from_index: usize,
    count: usize,
) {
    // 1. Assert: fromBlock and toBlock are distinct values.
    debug_assert!(unsafe {
        to_block.ptr.as_ptr().add(to_block.max_byte_length()) <= from_block.ptr.as_ptr()
            || from_block.ptr.as_ptr().add(from_block.max_byte_length()) <= to_block.ptr.as_ptr()
    });
    // 2. Let fromSize be the number of bytes in fromBlock.
    let from_size = from_block.max_byte_length();
    // 3. Assert: fromIndex + count ≤ fromSize.
    assert!(from_index + count <= from_size);
    // 4. Let toSize be the number of bytes in toBlock.
    let to_size = to_block.max_byte_length();
    // 5. Assert: toIndex + count ≤ toSize.
    assert!(to_index + count <= to_size);
    // 6. Repeat, while count > 0,
    // a. If fromBlock is a Shared Data Block, then
    // i. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
    // ii. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
    // iii. Let bytes be a List whose sole element is a nondeterministically chosen byte value.
    // iv. NOTE: In implementations, bytes is the result of a non-atomic read instruction on the underlying hardware. The nondeterminism is a semantic prescription of the memory model to describe observable behaviour of hardware with weak consistency.
    // v. Let readEvent be ReadSharedMemory { [[Order]]: UNORDERED, [[NoTear]]: true, [[Block]]: fromBlock, [[ByteIndex]]: fromIndex, [[ElementSize]]: 1 }.
    // vi. Append readEvent to eventsRecord.[[EventList]].
    // vii. Append Chosen Value Record { [[Event]]: readEvent, [[ChosenValue]]: bytes } to execution.[[ChosenValues]].
    // viii. If toBlock is a Shared Data Block, then
    //     1. Append WriteSharedMemory { [[Order]]: UNORDERED, [[NoTear]]: true, [[Block]]: toBlock, [[ByteIndex]]: toIndex, [[ElementSize]]: 1, [[Payload]]: bytes } to eventsRecord.[[EventList]].
    // ix. Else,
    //     1. Set toBlock[toIndex] to bytes[0].
    // Note: this can very well cause data races! That is language level UB in
    // Rust, so this is very much undefined behaviour _if_ the JavaScript code
    // causes a data race. The ECMAScript specification helpfully "recommends
    // programs be kept data races free". We'll trust that, I guess!?
    // SAFETY: Pointers have been checked to not overlap.
    to_block
        .as_racy_slice()
        .slice(to_index, to_index + count)
        .copy_from_racy_slice(
            &from_block
                .as_racy_slice()
                .slice(from_index, from_index + count),
        );
    // 7. Return UNUSED.
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct U8Clamped(pub u8);

impl core::fmt::Debug for U8Clamped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

mod private {
    use super::U8Clamped;

    pub trait Sealed {}
    impl Sealed for () {}
    impl Sealed for u8 {}
    impl Sealed for U8Clamped {}
    impl Sealed for i8 {}
    impl Sealed for u16 {}
    impl Sealed for i16 {}
    impl Sealed for u32 {}
    impl Sealed for i32 {}
    impl Sealed for u64 {}
    impl Sealed for i64 {}
    #[cfg(feature = "proposal-float16array")]
    impl Sealed for f16 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

pub trait Viewable: 'static + private::Sealed + Copy + PartialEq + core::fmt::Debug {
    /// Type of the data in its storage format. This is used with
    /// SharedDataBlock.
    type Storage: RacyStorage;

    /// Functions as the \[\[ContentType\]\] internal slot of the TypedArray and
    /// as a marker for data views. Used to determine that the viewable type is
    /// a BigInt.
    const IS_BIGINT: bool = false;
    const IS_FLOAT: bool = false;
    const NAME: &str;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics;

    fn into_be_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a>;
    fn into_le_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a>;
    #[inline(always)]
    fn into_ne_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a> {
        if cfg!(target_endian = "little") {
            self.into_le_value(agent, gc)
        } else {
            self.into_be_value(agent, gc)
        }
    }
    fn from_le_value(agent: &Agent, value: Numeric) -> Self;
    fn from_be_value(agent: &Agent, value: Numeric) -> Self;
    #[inline(always)]
    fn from_ne_value(agent: &Agent, value: Numeric) -> Self {
        if cfg!(target_endian = "little") {
            Self::from_le_value(agent, value)
        } else {
            Self::from_be_value(agent, value)
        }
    }
    /// Try reinterpret a Value to Viewable.
    ///
    /// This method is intended for cases where the ECMAScript specification
    /// requires repeatedly converting existing Viewable values to Value and
    /// comparing them with a search element. In this case, the Viewable to
    /// Value conversion is lossless and the comparison function is the only
    /// place where some value coercion may happen; this is generally the -0.0
    /// value being coerced to 0.
    ///
    /// Thus, this method must not do conversion, rounding, or clamping of
    /// numeric values.
    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self>;
    fn default() -> Self;

    /// Convert a Viewable value into a u64 holding an integer.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn into_bits(self) -> u64;
    /// Convert a u64 holding an integer into a Viewable.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn from_bits(bits: u64) -> Self;

    fn into_storage(value: Self) -> Self::Storage;

    fn from_storage(value: Self::Storage) -> Self;

    /// Convert a Viewable value into an f64.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn into_f64(self) -> f64;
    /// Convert an f64 into a Viewable.
    ///
    /// This is used to convert Viewables to other Viewables without having to
    /// go through a conversion into Value.
    fn from_f64(value: f64) -> Self;

    /// Reverses the byte order of the value.
    fn flip_endian(self) -> Self;

    fn neg(self) -> Self;
    fn add(self, other: Self) -> Self;
    fn and(self, other: Self) -> Self;
    fn swap(self, other: Self) -> Self;
    fn or(self, other: Self) -> Self;
    fn sub(self, other: Self) -> Self;
    fn xor(self, other: Self) -> Self;

    /// Compare A and B of a Viewable type and always return an Ordering.
    ///
    /// This ordering is the usual total order for integers, and the special
    /// ECMAScript defined ordering for floats.
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering;
}

impl Viewable for () {
    // Note: this is not a valid storage format for () really. VoidArrays never
    // get stored so it's okay.
    type Storage = u8;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint8Array;
    const NAME: &str = "VoidArray";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        panic!("VoidArray is a marker type");
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        panic!("VoidArray is a marker type");
    }

    fn from_be_value(_: &Agent, _: Numeric) -> Self {
        panic!("VoidArray is a marker type");
    }

    fn from_le_value(_: &Agent, _: Numeric) -> Self {
        panic!("VoidArray is a marker type");
    }

    fn try_from_value(_: &mut Agent, _: Value) -> Option<Self> {
        panic!("VoidArray is a marker type");
    }

    fn default() -> Self {
        panic!("VoidArray is a marker type");
    }

    fn into_bits(self) -> u64 {
        panic!("VoidArray is a marker type");
    }

    fn from_bits(_: u64) -> Self {
        panic!("VoidArray is a marker type");
    }

    fn from_storage(_: Self::Storage) -> Self {
        panic!("VoidArray is a marker type");
    }

    fn into_storage(_: Self) -> Self::Storage {
        panic!("VoidArray is a marker type");
    }

    fn into_f64(self) -> f64 {
        panic!("VoidArray is a marker type");
    }

    fn from_f64(_: f64) -> Self {
        panic!("VoidArray is a marker type");
    }

    fn flip_endian(self) -> Self {
        panic!("VoidArray is a marker type");
    }

    fn neg(self) -> Self {
        panic!("VoidArray is a marker type");
    }
    fn add(self, _: Self) -> Self {
        panic!("VoidArray is a marker type");
    }
    fn and(self, _: Self) -> Self {
        panic!("VoidArray is a marker type");
    }
    fn swap(self, _: Self) -> Self {
        panic!("VoidArray is a marker type");
    }
    fn or(self, _: Self) -> Self {
        panic!("VoidArray is a marker type");
    }
    fn sub(self, _: Self) -> Self {
        panic!("VoidArray is a marker type");
    }
    fn xor(self, _: Self) -> Self {
        panic!("VoidArray is a marker type");
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}

impl Viewable for u8 {
    type Storage = Self;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint8Array;
    const NAME: &str = "Uint8Array";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint8_number(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint8_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        u8::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self.into()
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for U8Clamped {
    type Storage = u8;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint8ClampedArray;
    const NAME: &str = "Uint8ClampedArray";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.0.to_be()).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.0.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self(to_uint8_clamp_number(agent, value).to_be())
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self(to_uint8_clamp_number(agent, value).to_le())
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(U8Clamped(0));
            }
            return None;
        };
        u8::try_from(value.into_i64()).ok().map(U8Clamped)
    }

    fn default() -> Self {
        U8Clamped(0)
    }

    fn into_bits(self) -> u64 {
        self.0.into()
    }

    fn from_bits(bits: u64) -> Self {
        U8Clamped(bits.clamp(0, 255) as u8)
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        Self(value)
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value.0
    }

    fn into_f64(self) -> f64 {
        self.0.into()
    }

    fn from_f64(value: f64) -> Self {
        if value.is_nan() {
            return Self(0);
        }
        U8Clamped(value.clamp(0.0, 255.0).round_ties_even() as u8)
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        Self(self.0.swap_bytes())
    }

    #[inline(always)]
    fn neg(self) -> Self {
        Self(self.0.wrapping_neg())
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        Self(self.0.bitand(other.0))
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        Self(self.0.bitor(other.0))
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        Self(self.0.bitxor(other.0))
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for i8 {
    type Storage = u8;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int8Array;
    const NAME: &str = "Int8Array";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int8_number(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int8_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        i8::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value.cast_signed()
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value.cast_unsigned()
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for u16 {
    type Storage = Self;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint16Array;
    const NAME: &str = "Uint16Array";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint16_number(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint16_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        u16::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for i16 {
    type Storage = u16;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int16Array;
    const NAME: &str = "Int16Array";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int16_number(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int16_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        i16::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value.cast_signed()
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value.cast_unsigned()
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for u32 {
    type Storage = Self;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint32Array;
    const NAME: &str = "Uint32Array";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint32_number(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_uint32_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        u32::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for i32 {
    type Storage = u32;

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int32Array;
    const NAME: &str = "Int32Array";

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_be()).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int32_number(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        to_int32_number(agent, value).to_le()
    }

    fn try_from_value(_: &mut Agent, value: Value) -> Option<Self> {
        let Value::Integer(value) = value else {
            if value == Value::SmallF64((-0.0f32).into()) {
                return Some(0);
            }
            return None;
        };
        i32::try_from(value.into_i64()).ok()
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value.cast_signed()
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value.cast_unsigned()
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for u64 {
    type Storage = Self;

    const IS_BIGINT: bool = true;
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::BigUint64Array;
    const NAME: &str = "BigUint64Array";

    fn into_be_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_u64(agent, self.to_be()).into()
    }

    fn into_le_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_u64(agent, self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_uint64_big_int(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_uint64_big_int(agent, value).to_le()
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        if let Value::SmallBigInt(value) = value {
            let value = value.into_i64();
            return u64::try_from(value).ok();
        };
        if let Value::BigInt(value) = value {
            let data = value.get(agent);
            let mut iter = data.data.iter_u64_digits();
            let sign = data.data.sign();
            if sign == Sign::Minus {
                return None;
            }
            if iter.len() > 1 {
                return None;
            }
            let value = iter.next().unwrap();
            return Some(value);
        };
        None
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self
    }

    fn from_bits(bits: u64) -> Self {
        bits
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value
    }

    fn into_f64(self) -> f64 {
        self as f64
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
impl Viewable for i64 {
    type Storage = u64;

    const IS_BIGINT: bool = true;
    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::BigInt64Array;
    const NAME: &str = "BigInt64Array";

    fn into_be_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_i64(agent, self.to_be()).into()
    }

    fn into_le_value<'a>(self, agent: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        BigInt::from_i64(agent, self.to_le()).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_int64_big_int(agent, value).to_be()
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = BigInt::try_from(value) else {
            unreachable!()
        };
        to_big_int64_big_int(agent, value).to_le()
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        if let Value::SmallBigInt(value) = value {
            return Some(value.into_i64());
        };
        if let Value::BigInt(value) = value {
            let data = value.get(agent);
            let mut iter = data.data.iter_u64_digits();
            if iter.len() > 1 {
                return None;
            }
            let sign = data.data.sign();
            let value = iter.next().unwrap();
            if sign == Sign::Minus {
                if value <= i64::MIN.unsigned_abs() {
                    return Some(value.wrapping_neg() as i64);
                } else {
                    return None;
                }
            } else if value <= i64::MAX as u64 {
                return Some(value as i64);
            } else {
                return None;
            }
        };
        None
    }

    fn default() -> Self {
        0
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        value.cast_signed()
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        value.cast_unsigned()
    }

    fn into_f64(self) -> f64 {
        self as f64
    }

    fn from_f64(value: f64) -> Self {
        if !value.is_finite() {
            return 0;
        }
        value.trunc() as i64 as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        self.swap_bytes()
    }

    #[inline(always)]
    fn neg(self) -> Self {
        self.wrapping_neg()
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self.wrapping_add(other)
    }
    #[inline(always)]
    fn and(self, other: Self) -> Self {
        self.bitand(other)
    }
    #[inline(always)]
    fn swap(self, other: Self) -> Self {
        other
    }
    #[inline(always)]
    fn or(self, other: Self) -> Self {
        self.bitor(other)
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self.wrapping_sub(other)
    }
    #[inline(always)]
    fn xor(self, other: Self) -> Self {
        self.bitxor(other)
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.cmp(other)
    }
}
#[cfg(feature = "proposal-float16array")]
impl Viewable for f16 {
    type Storage = u16;

    const IS_FLOAT: bool = true;
    const NAME: &str = "Float16Array";

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float16Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_be_bytes())).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_le_bytes())).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_be_bytes())
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_le_bytes())
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        let Ok(value) = Number::try_from(value) else {
            return None;
        };
        let value = value.into_f64(agent);
        if value.is_nan() {
            return Some(f16::NAN);
        }
        if value as f16 as f64 == value {
            Some(value as f16)
        } else {
            None
        }
    }
    fn default() -> Self {
        f16::NAN
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        f16::from_bits(value)
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        f16::to_bits(value)
    }

    fn into_f64(self) -> f64 {
        self as f64
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        Self::from_bits(self.to_bits().swap_bytes())
    }

    #[inline(always)]
    fn neg(self) -> Self {
        core::ops::Neg::neg(self)
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self + other
    }
    #[inline(always)]
    fn and(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn swap(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn or(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self - other
    }
    #[inline(always)]
    fn xor(self, _other: Self) -> Self {
        unreachable!()
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.is_nan() {
            if other.is_nan() {
                return core::cmp::Ordering::Equal;
            }
            return core::cmp::Ordering::Greater;
        }
        if other.is_nan() {
            return core::cmp::Ordering::Less;
        }
        if *self == 0.0 && *other == 0.0 {
            if self.is_sign_negative() && other.is_sign_positive() {
                return core::cmp::Ordering::Less;
            }
            if self.is_sign_positive() && other.is_sign_negative() {
                return core::cmp::Ordering::Greater;
            }
            return core::cmp::Ordering::Equal;
        }
        self.partial_cmp(other).unwrap()
    }
}
impl Viewable for f32 {
    type Storage = u32;

    const IS_FLOAT: bool = true;
    const NAME: &str = "Float32Array";

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float32Array;

    fn into_be_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_be_bytes())).into()
    }

    fn into_le_value<'a>(self, _: &mut Agent, _: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from(Self::from_ne_bytes(self.to_le_bytes())).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_be_bytes())
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_le_bytes())
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        let Ok(value) = Number::try_from(value) else {
            return None;
        };
        let value = value.into_f64(agent);
        if value.is_nan() {
            return Some(f32::NAN);
        }
        if value as f32 as f64 == value {
            Some(value as f32)
        } else {
            None
        }
    }

    fn default() -> Self {
        f32::NAN
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        f32::from_bits(value)
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        f32::to_bits(value)
    }

    fn into_f64(self) -> f64 {
        self.into()
    }

    fn from_f64(value: f64) -> Self {
        value as Self
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        Self::from_bits(self.to_bits().swap_bytes())
    }

    #[inline(always)]
    fn neg(self) -> Self {
        core::ops::Neg::neg(self)
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self + other
    }
    #[inline(always)]
    fn and(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn swap(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn or(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self - other
    }
    #[inline(always)]
    fn xor(self, _other: Self) -> Self {
        unreachable!()
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.is_nan() {
            if other.is_nan() {
                return core::cmp::Ordering::Equal;
            }
            return core::cmp::Ordering::Greater;
        }
        if other.is_nan() {
            return core::cmp::Ordering::Less;
        }
        if *self == 0.0 && *other == 0.0 {
            if self.is_sign_negative() && other.is_sign_positive() {
                return core::cmp::Ordering::Less;
            }
            if self.is_sign_positive() && other.is_sign_negative() {
                return core::cmp::Ordering::Greater;
            }
            return core::cmp::Ordering::Equal;
        }
        self.partial_cmp(other).unwrap()
    }
}
impl Viewable for f64 {
    type Storage = u64;

    const IS_FLOAT: bool = true;
    const NAME: &str = "Float64Array";

    #[cfg(feature = "array-buffer")]
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float64Array;

    fn into_be_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from_f64(agent, Self::from_ne_bytes(self.to_be_bytes()), gc).into()
    }

    fn into_le_value<'a>(self, agent: &mut Agent, gc: NoGcScope<'a, '_>) -> Numeric<'a> {
        Number::from_f64(agent, Self::from_ne_bytes(self.to_le_bytes()), gc).into()
    }

    fn from_be_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_be_bytes())
    }

    fn from_le_value(agent: &Agent, value: Numeric) -> Self {
        let Ok(value) = Number::try_from(value) else {
            unreachable!()
        };
        Self::from_ne_bytes((value.to_real(agent) as Self).to_le_bytes())
    }

    fn try_from_value(agent: &mut Agent, value: Value) -> Option<Self> {
        let Ok(value) = Number::try_from(value) else {
            return None;
        };
        Some(value.into_f64(agent))
    }

    fn default() -> Self {
        f64::NAN
    }

    fn into_bits(self) -> u64 {
        self as u64
    }

    fn from_bits(bits: u64) -> Self {
        bits as Self
    }

    #[inline(always)]
    fn from_storage(value: Self::Storage) -> Self {
        f64::from_bits(value)
    }

    #[inline(always)]
    fn into_storage(value: Self) -> Self::Storage {
        f64::to_bits(value)
    }

    fn into_f64(self) -> f64 {
        self
    }

    fn from_f64(value: f64) -> Self {
        value
    }

    #[inline(always)]
    fn flip_endian(self) -> Self {
        Self::from_bits(self.to_bits().swap_bytes())
    }

    #[inline(always)]
    fn neg(self) -> Self {
        core::ops::Neg::neg(self)
    }
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        self + other
    }
    #[inline(always)]
    fn and(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn swap(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn or(self, _other: Self) -> Self {
        unreachable!()
    }
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        self - other
    }
    #[inline(always)]
    fn xor(self, _other: Self) -> Self {
        unreachable!()
    }

    #[inline(always)]
    fn ecmascript_cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.is_nan() {
            if other.is_nan() {
                return core::cmp::Ordering::Equal;
            }
            return core::cmp::Ordering::Greater;
        }
        if other.is_nan() {
            return core::cmp::Ordering::Less;
        }
        if *self == 0.0 && *other == 0.0 {
            if self.is_sign_negative() && other.is_sign_positive() {
                return core::cmp::Ordering::Less;
            }
            if self.is_sign_positive() && other.is_sign_negative() {
                return core::cmp::Ordering::Greater;
            }
            return core::cmp::Ordering::Equal;
        }
        self.partial_cmp(other).unwrap()
    }
}

mod tests {
    #[test]
    fn new_data_block() {
        use super::DataBlock;
        let db = DataBlock::new(0).unwrap();
        assert_eq!(db.len(), 0);
        assert_eq!(db.get::<u8>(0), None);

        let db = DataBlock::new(8).unwrap();
        assert_eq!(db.len(), 8);
        for i in 0..8 {
            assert_eq!(db.get::<u8>(i), Some(0));
        }
    }

    #[test]
    fn data_block_set() {
        use super::DataBlock;
        let mut db = DataBlock::new(8).unwrap();
        assert_eq!(db.len(), 8);
        for i in 0..8 {
            assert_eq!(db.get::<u8>(i), Some(0));
        }

        for i in 0..8 {
            db.set::<u8>(i as usize, i + 1);
        }

        for i in 0..8 {
            assert_eq!(db.get::<u8>(i as usize), Some(i + 1));
        }
    }

    #[test]
    fn data_block_set_from() {
        use super::DataBlock;
        let mut db = DataBlock::new(8).unwrap();
        let mut db2 = DataBlock::new(8).unwrap();
        for i in 0..8 {
            assert_eq!(db.get::<u8>(0), Some(0));
            db2.set::<u8>(i as usize, i + 1);
        }
        assert_eq!(db2.get::<u8>(0), Some(1));
        assert_eq!(db2.get::<u8>(1), Some(2));
        assert_eq!(db2.get::<u8>(2), Some(3));
        assert_eq!(db2.get::<u8>(3), Some(4));
        assert_eq!(db2.get::<u8>(4), Some(5));
        assert_eq!(db2.get::<u8>(5), Some(6));
        assert_eq!(db2.get::<u8>(6), Some(7));
        assert_eq!(db2.get::<u8>(7), Some(8));
        db.set_from::<u8>(0, &db2, 4, 4);
        assert_eq!(db.get::<u8>(0), Some(5));
        assert_eq!(db.get::<u8>(1), Some(6));
        assert_eq!(db.get::<u8>(2), Some(7));
        assert_eq!(db.get::<u8>(3), Some(8));
        assert_eq!(db.get::<u8>(4), Some(0));
        assert_eq!(db.get::<u8>(5), Some(0));
        assert_eq!(db.get::<u8>(6), Some(0));
        assert_eq!(db.get::<u8>(7), Some(0));

        // Reset
        for i in 0..8 {
            db.set::<u8>(i as usize, i + 1);
        }
        db.copy_within::<u8>(2, 4, 4);
        assert_eq!(db.get::<u8>(0), Some(1));
        assert_eq!(db.get::<u8>(1), Some(2));
        assert_eq!(db.get::<u8>(2), Some(5));
        assert_eq!(db.get::<u8>(3), Some(6));
        assert_eq!(db.get::<u8>(4), Some(7));
        assert_eq!(db.get::<u8>(5), Some(8));
        assert_eq!(db.get::<u8>(6), Some(7));
        assert_eq!(db.get::<u8>(7), Some(8));
    }

    #[test]
    fn data_block_copy_within() {
        use super::DataBlock;
        let mut db = DataBlock::new(8).unwrap();
        for i in 0..8 {
            db.set::<u8>(i as usize, i + 1);
        }
        assert_eq!(db.get::<u8>(0), Some(1));
        assert_eq!(db.get::<u8>(1), Some(2));
        assert_eq!(db.get::<u8>(2), Some(3));
        assert_eq!(db.get::<u8>(3), Some(4));
        assert_eq!(db.get::<u8>(4), Some(5));
        assert_eq!(db.get::<u8>(5), Some(6));
        assert_eq!(db.get::<u8>(6), Some(7));
        assert_eq!(db.get::<u8>(7), Some(8));
        db.copy_within::<u8>(0, 4, 4);
        assert_eq!(db.get::<u8>(0), Some(5));
        assert_eq!(db.get::<u8>(1), Some(6));
        assert_eq!(db.get::<u8>(2), Some(7));
        assert_eq!(db.get::<u8>(3), Some(8));
        assert_eq!(db.get::<u8>(4), Some(5));
        assert_eq!(db.get::<u8>(5), Some(6));
        assert_eq!(db.get::<u8>(6), Some(7));
        assert_eq!(db.get::<u8>(7), Some(8));

        // Reset
        for i in 0..8 {
            db.set::<u8>(i as usize, i + 1);
        }
        db.copy_within::<u8>(2, 4, 4);
        assert_eq!(db.get::<u8>(0), Some(1));
        assert_eq!(db.get::<u8>(1), Some(2));
        assert_eq!(db.get::<u8>(2), Some(5));
        assert_eq!(db.get::<u8>(3), Some(6));
        assert_eq!(db.get::<u8>(4), Some(7));
        assert_eq!(db.get::<u8>(5), Some(8));
        assert_eq!(db.get::<u8>(6), Some(7));
        assert_eq!(db.get::<u8>(7), Some(8));
    }

    #[test]
    #[cfg(feature = "shared-array-buffer")]
    fn new_shared_data_block() {
        use super::SharedDataBlock;
        // SAFETY: max_byte_length is None.
        let _ = unsafe { SharedDataBlock::new(1024, None).unwrap() };
    }

    #[test]
    #[cfg(feature = "shared-array-buffer")]
    fn clone_shared_data_block() {
        use super::SharedDataBlock;
        // SAFETY: max_byte_length is None.
        let a = unsafe { SharedDataBlock::new(1024, None).unwrap() };
        let _ = a.clone();
    }

    #[test]
    #[cfg(feature = "shared-array-buffer")]
    fn clone_shared_data_block_thread_safe() {
        use super::SharedDataBlock;
        use std::thread;
        // SAFETY: max_byte_length is None.
        let a = unsafe { SharedDataBlock::new(1024, None).unwrap() };
        thread::scope(|s| {
            s.spawn(|| a.clone());
            s.spawn(|| a.clone());
            s.spawn(|| a.clone());
            s.spawn(|| a.clone());
        });
    }
}
