//! ### [6.2.9 Data Blocks](https://tc39.es/ecma262/#sec-data-blocks)

use std::{
    alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout},
    ptr::{read_unaligned, write_bytes, write_unaligned, NonNull},
};

use crate::ecmascript::execution::{agent::JsError, Agent, JsResult};

/// # Data Block
///
/// The Data Block specification type is used to describe a distinct and
/// mutable sequence of byte-sized (8 bit) numeric values. A byte value
/// is an integer in the inclusive interval from 0 to 255. A Data Block
/// value is created with a fixed number of bytes that each have the
/// initial value 0.
///
/// The `ptr` points to a continuous buffer
/// of bytes, the length of which is determined by
/// the capacity. The pointer can be None if the
/// capacity of the buffer is zero.
#[derive(Debug, Clone)]
pub(crate) struct DataBlock {
    ptr: Option<NonNull<u8>>,
    cap: u32,
    byte_length: u32,
}

impl Drop for DataBlock {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            let layout = Layout::from_size_align(self.cap as usize, 8).unwrap();
            unsafe { dealloc(ptr.as_ptr(), layout) }
        }
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for i8 {}
    impl Sealed for u16 {}
    impl Sealed for i16 {}
    impl Sealed for u32 {}
    impl Sealed for i32 {}
    impl Sealed for u64 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

pub trait Viewable: private::Sealed {}

impl Viewable for u8 {}
impl Viewable for i8 {}
impl Viewable for u16 {}
impl Viewable for i16 {}
impl Viewable for u32 {}
impl Viewable for i32 {}
impl Viewable for u64 {}
impl Viewable for i64 {}
impl Viewable for f32 {}
impl Viewable for f64 {}

impl DataBlock {
    fn new(len: u32) -> Self {
        let ptr = if len == 0 {
            None
        } else {
            let layout = Layout::from_size_align(len as usize, 8).unwrap();
            // SAFETY: Size of allocation is non-zero.
            let data = unsafe { alloc_zeroed(layout) };
            if data.is_null() {
                // TODO: Throw error?
                handle_alloc_error(layout);
            }
            debug_assert_eq!(data.align_offset(8), 0);
            NonNull::new(data)
        };
        Self {
            ptr,
            cap: len,
            byte_length: len,
        }
    }

    pub fn new_with_capacity(len: u32, cap: u32) -> Self {
        debug_assert!(cap >= len);
        let ptr = if cap == 0 {
            None
        } else {
            let layout = Layout::from_size_align(cap as usize, 8).unwrap();
            // SAFETY: Size of allocation is non-zero.
            let data = unsafe { alloc_zeroed(layout) };
            if data.is_null() {
                handle_alloc_error(layout);
            }
            debug_assert_eq!(data.align_offset(8), 0);
            NonNull::new(data)
        };
        Self {
            ptr,
            cap,
            byte_length: len,
        }
    }

    pub fn len(&self) -> u32 {
        self.byte_length
    }

    pub fn capacity(&self) -> u32 {
        self.cap
    }

    pub fn resize(&mut self, size: u32) {
        debug_assert!(size <= self.cap);
        let len = self.byte_length;
        self.byte_length = size;
        if size < len {
            // Zero out the "dropped" bytes.
            if let Some(data) = self.ptr {
                // SAFETY: The data is properly initialized, and the T being written is
                // checked to be fully within the length of the data allocation.
                unsafe { write_bytes(data.as_ptr().add(size as usize), 0, (len - size) as usize) }
            }
        }
    }

    pub fn view_len<T: Viewable>(&self, byte_offset: u32) -> u32 {
        let size = std::mem::size_of::<T>() as u32;
        (self.byte_length - byte_offset) / size
    }

    fn as_ptr(&self, byte_offset: u32) -> Option<*const u8> {
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { data.as_ptr().add(byte_offset as usize) as *const _ }
            })
        }
    }

    fn as_mut_ptr(&mut self, byte_offset: u32) -> Option<*mut u8> {
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { data.as_ptr().add(byte_offset as usize) }
            })
        }
    }

    pub fn get<T: Viewable>(&self, offset: u32) -> Option<T> {
        let size = std::mem::size_of::<T>() as u32;
        let byte_offset = offset * size;
        if byte_offset >= self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { read_unaligned(data.as_ptr().add(offset as usize).cast()) }
            })
        }
    }

    pub fn set<T: Viewable>(&mut self, offset: u32, value: T) {
        let size = std::mem::size_of::<T>() as u32;
        let byte_offset = offset * size;
        if let Some(data) = self.ptr {
            if byte_offset <= self.byte_length {
                // SAFETY: The data is properly initialized, and the T being written is
                // checked to be fully within the length of the data allocation.
                unsafe { write_unaligned(data.as_ptr().add(offset as usize).cast(), value) }
            }
        }
    }

    pub fn set_from<T: Viewable>(
        &mut self,
        dst_offset: u32,
        src: &DataBlock,
        src_offset: u32,
        count: u32,
    ) {
        let size = std::mem::size_of::<T>() as u32;
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
            unsafe { dst.copy_from_nonoverlapping(src, byte_length as usize) }
        }
    }

    pub fn copy_within<T: Viewable>(&mut self, dst_offset: u32, src_offset: u32, count: u32) {
        let size = std::mem::size_of::<T>() as u32;
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
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, self.byte_length as usize) };
            slice.copy_within(
                (src_byte_offset as usize)..(src_byte_offset + byte_length) as usize,
                dst_byte_offset as usize,
            );
        }
    }

    /// ### [6.2.9.1 CreateByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createbytedatablock)
    ///
    /// The abstract operation CreateByteDataBlock takes argument size (a non-negative integer)
    /// and returns either a normal completion containing a Data Block or a throw completion.
    pub fn create_byte_data_block(_agent: &Agent, size: u64) -> JsResult<Self> {
        // 1. If size > 2**53 - 1, throw a RangeError exception.
        if size > u64::pow(2, 53) - 1 {
            // TODO: throw a RangeError exception
            Err(JsError {})
        } else if let Ok(size) = u32::try_from(size) {
            // 2. Let db be a new Data Block value consisting of size bytes.
            // 3. Set all of the bytes of db to 0.
            // 4. Return db.
            Ok(Self::new(size))
        } else {
            // 2. cont: If it is impossible to create such a Data Block, throw a RangeError exception.
            // TODO: throw a RangeError exception
            Err(JsError {})
        }
    }

    /// ### [6.2.9.2 CreateSharedByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createsharedbytedatablock)
    ///
    /// The abstract operation CreateSharedByteDataBlock takes argument size (a non-negative integer)
    /// and returns either a normal completion containing a Shared Data Block or a throw completion.
    pub fn create_shared_byte_data_block(size: u64) -> JsResult<Self> {
        // 1. Let db be a new Shared Data Block value consisting of size bytes. If it is impossible to create such a Shared Data Block, throw a RangeError exception.
        if let Ok(size) = u32::try_from(size) {
            // 2. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
            // 3. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
            // 4. Let zero be « 0 ».
            // 5. For each index i of db, do
            // a. Append WriteSharedMemory { [[Order]]: INIT, [[NoTear]]: true, [[Block]]: db, [[ByteIndex]]: i, [[ElementSize]]: 1, [[Payload]]: zero } to eventsRecord.[[EventList]].
            Ok(Self::new(size))
        } else {
            Err(JsError {})
        }
        // 6. Return db.
    }

    /// ### [6.2.9.3 CopyDataBlockBytes ( toBlock, toIndex, fromBlock, fromIndex, count )](https://tc39.es/ecma262/#sec-copydatablockbytes)
    ///
    /// The abstract operation CopyDataBlockBytes takes arguments toBlock (a Data Block or a Shared
    /// Data Block), toIndex (a non-negative integer), fromBlock (a Data Block or a Shared Data Block),
    /// fromIndex (a non-negative integer), and count (a non-negative integer) and returns UNUSED.
    pub fn copy_data_block_bytes(
        &mut self,
        to_index: u32,
        from_block: &Self,
        from_index: u32,
        count: u32,
    ) {
        let to_block = self;
        // 1. Assert: fromBlock and toBlock are distinct values.
        assert!(
            to_block.ptr.is_none()
                || from_block.ptr.is_none()
                || unsafe {
                    to_block
                        .ptr
                        .unwrap()
                        .as_ptr()
                        .add(to_block.capacity() as usize)
                        <= from_block.ptr.unwrap().as_ptr()
                        || from_block
                            .ptr
                            .unwrap()
                            .as_ptr()
                            .add(from_block.capacity() as usize)
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
        //      a. If fromBlock is a Shared Data Block, then
        //          i. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
        //          ii. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
        //          iii. Let bytes be a List whose sole element is a nondeterministically chosen byte value.
        //          iv. NOTE: In implementations, bytes is the result of a non-atomic read instruction on the underlying hardware. The nondeterminism is a semantic prescription of the memory model to describe observable behaviour of hardware with weak consistency.
        //          v. Let readEvent be ReadSharedMemory { [[Order]]: UNORDERED, [[NoTear]]: true, [[Block]]: fromBlock, [[ByteIndex]]: fromIndex, [[ElementSize]]: 1 }.
        //          vi. Append readEvent to eventsRecord.[[EventList]].
        //          vii. Append Chosen Value Record { [[Event]]: readEvent, [[ChosenValue]]: bytes } to execution.[[ChosenValues]].
        //          viii. If toBlock is a Shared Data Block, then
        //              1. Append WriteSharedMemory { [[Order]]: UNORDERED, [[NoTear]]: true, [[Block]]: toBlock, [[ByteIndex]]: toIndex, [[ElementSize]]: 1, [[Payload]]: bytes } to eventsRecord.[[EventList]].
        //          ix. Else,
        //              1. Set toBlock[toIndex] to bytes[0].
        //      b. Else,
        //           i. Assert: toBlock is not a Shared Data Block.
        //           ii. Set toBlock[toIndex] to fromBlock[fromIndex].
        //           c. Set toIndex to toIndex + 1.
        //           d. Set fromIndex to fromIndex + 1.
        //           e. Set count to count - 1.
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
        unsafe { to_ptr.copy_from_nonoverlapping(from_ptr, count as usize) };
        // 7. Return UNUSED.
    }
}

#[test]
fn new_data_block() {
    let db = DataBlock::new(0);
    assert_eq!(db.len(), 0);
    assert_eq!(db.capacity(), 0);
    assert_eq!(db.get::<u8>(0), None);

    let db = DataBlock::new(8);
    assert_eq!(db.len(), 8);
    assert_eq!(db.capacity(), 8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i as u32), Some(0));
    }
}

#[test]
fn new_data_block_with_capacity() {
    let db = DataBlock::new_with_capacity(0, 8);
    assert_eq!(db.len(), 0);
    assert_eq!(db.capacity(), 8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i as u32), None);
    }

    let db = DataBlock::new_with_capacity(8, 16);
    assert_eq!(db.len(), 8);
    assert_eq!(db.capacity(), 16);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i as u32), Some(0));
    }
    for i in 8..16 {
        assert_eq!(db.get::<u8>(i as u32), None);
    }
}

#[test]
fn data_block_set() {
    let mut db = DataBlock::new(8);
    assert_eq!(db.len(), 8);
    assert_eq!(db.capacity(), 8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i as u32), Some(0));
    }

    for i in 0..8 {
        db.set::<u8>(i as u32, i + 1);
    }

    for i in 0..8 {
        assert_eq!(db.get::<u8>(i as u32), Some(i + 1));
    }
}

#[test]
fn data_block_resize() {
    let mut db = DataBlock::new_with_capacity(0, 8);
    db.resize(8);
    assert_eq!(db.len(), 8);
    assert_eq!(db.capacity(), 8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i as u32), Some(0));
    }

    for i in 0..8 {
        db.set::<u8>(i as u32, i + 1);
    }

    let ptr = db.as_ptr(0).unwrap();
    db.resize(0);

    // SAFETY: Backing store is not deallocated: Zero index pointer is still valid
    // and is not read beyond its allocated capacity. The only usual safety requirement
    // broken is to read beyond the buffer length, which is safe as the outside is still
    // properly initialized u8s.
    unsafe {
        for i in 0..8 {
            assert_eq!(ptr.add(i).read(), 0);
        }
    }
}

#[test]
fn data_block_set_from() {
    let mut db = DataBlock::new(8);
    let mut db2 = DataBlock::new(8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(0), Some(0));
        db2.set::<u8>(i as u32, i + 1);
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
        db.set::<u8>(i as u32, i + 1);
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
    let mut db = DataBlock::new(8);
    for i in 0..8 {
        db.set::<u8>(i as u32, i + 1);
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
        db.set::<u8>(i as u32, i + 1);
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
