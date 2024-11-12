// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [6.2.9 Data Blocks](https://tc39.es/ecma262/#sec-data-blocks)

use std::{
    alloc::{alloc_zeroed, dealloc, handle_alloc_error, realloc, Layout},
    ptr::{self, read_unaligned, write_unaligned, NonNull},
};

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{
            to_big_int64, to_big_uint64, to_int16, to_int32, to_int8, to_uint16, to_uint32,
            to_uint8, to_uint8_clamp,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics},
        types::{BigInt, IntoValue, Value},
    },
    engine::context::GcScope,
};

/// Sentinel pointer for a detached data block.
///
/// We allocate at 8 byte alignment so this is never a valid DataBlock pointer normally.
const DETACHED_DATA_BLOCK_POINTER: *mut u8 = 0xde7ac4ed as *mut u8;

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
    byte_length: usize,
}

impl Drop for DataBlock {
    fn drop(&mut self) {
        if let Some(ptr) = self.ptr {
            if ptr::eq(ptr.as_ptr(), DETACHED_DATA_BLOCK_POINTER) {
                // Don't try to dealloc a detached data block.
                return;
            }
            let layout = Layout::from_size_align(self.byte_length, 8).unwrap();
            unsafe { dealloc(ptr.as_ptr(), layout) }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct U8Clamped(pub u8);

mod private {
    use super::U8Clamped;

    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for U8Clamped {}
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

pub trait Viewable: private::Sealed + Copy {
    /// Functions as the \[\[ContentType\]\] internal slot of the TypedArray and
    /// as a marker for data views. Used to determine that the viewable type is
    /// a BigInt.
    const IS_BIGINT: bool;
    const PROTO: ProtoIntrinsics;

    fn into_be_value(self, agent: &mut Agent) -> Value;
    fn into_le_value(self, agent: &mut Agent) -> Value;
    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self;
    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self;

    // TODO: Consider adding the following methods if needed
    // fn into_ne_value(self, agent: &mut Agent) -> Value;
    // fn from_ne_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self;
}

impl Viewable for u8 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint8Array;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_be())
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_le())
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_uint8(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_uint8(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for U8Clamped {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint8ClampedArray;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(self.0.to_be())
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(self.0.to_le())
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        Self(to_uint8_clamp(agent, gc, value).unwrap().to_be())
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        Self(to_uint8_clamp(agent, gc, value).unwrap().to_le())
    }
}
impl Viewable for i8 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int8Array;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_be())
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_le())
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_int8(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_int8(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for u16 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint16Array;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_be())
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_le())
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_uint16(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_uint16(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for i16 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int16Array;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_be())
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_le())
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_int16(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_int16(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for u32 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Uint32Array;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_be())
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_le())
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_uint32(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_uint32(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for i32 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Int32Array;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_be())
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(self.to_le())
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_int32(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_int32(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for u64 {
    const IS_BIGINT: bool = true;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::BigUint64Array;

    fn into_be_value(self, agent: &mut Agent) -> Value {
        BigInt::from_u64(agent, self.to_be()).into_value()
    }

    fn into_le_value(self, agent: &mut Agent) -> Value {
        BigInt::from_u64(agent, self.to_le()).into_value()
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_big_uint64(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_big_uint64(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for i64 {
    const IS_BIGINT: bool = true;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::BigInt64Array;

    fn into_be_value(self, agent: &mut Agent) -> Value {
        BigInt::from_i64(agent, self.to_be()).into_value()
    }

    fn into_le_value(self, agent: &mut Agent) -> Value {
        BigInt::from_i64(agent, self.to_le()).into_value()
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_big_int64(agent, gc, value).unwrap().to_be()
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        to_big_int64(agent, gc, value).unwrap().to_le()
    }
}
impl Viewable for f32 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float32Array;

    fn into_be_value(self, _: &mut Agent) -> Value {
        Value::from(Self::from_ne_bytes(self.to_be_bytes()))
    }

    fn into_le_value(self, _: &mut Agent) -> Value {
        Value::from(Self::from_ne_bytes(self.to_le_bytes()))
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        Self::from_ne_bytes((value.to_real(agent, gc).unwrap() as Self).to_be_bytes())
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        Self::from_ne_bytes((value.to_real(agent, gc).unwrap() as Self).to_le_bytes())
    }
}
impl Viewable for f64 {
    const IS_BIGINT: bool = false;
    const PROTO: ProtoIntrinsics = ProtoIntrinsics::Float64Array;

    fn into_be_value(self, agent: &mut Agent) -> Value {
        Value::from_f64(agent, Self::from_ne_bytes(self.to_be_bytes()))
    }

    fn into_le_value(self, agent: &mut Agent) -> Value {
        Value::from_f64(agent, Self::from_ne_bytes(self.to_le_bytes()))
    }

    fn from_be_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        Self::from_ne_bytes((value.to_real(agent, gc).unwrap() as Self).to_be_bytes())
    }

    fn from_le_value(agent: &mut Agent, gc: GcScope<'_, '_>, value: Value) -> Self {
        Self::from_ne_bytes((value.to_real(agent, gc).unwrap() as Self).to_le_bytes())
    }
}

impl DataBlock {
    /// Sentinel value for detached DataBlocks.
    ///
    /// This sentinel value is never safe to read from or write data to. The
    /// length is 0 so it shouldn't be possible to either.
    pub const DETACHED_DATA_BLOCK: DataBlock = DataBlock {
        // SAFETY: 0xde7ac4ed is not 0. Note that we always allocate at 8 byte
        // alignment, so a DataBlock pointer cannot have this value naturally.
        ptr: Some(unsafe { NonNull::new_unchecked(DETACHED_DATA_BLOCK_POINTER) }),
        byte_length: 0,
    };

    pub fn is_detached(&self) -> bool {
        if let (Some(a), Some(b)) = (self.ptr, Self::DETACHED_DATA_BLOCK.ptr) {
            ptr::eq(a.as_ptr(), b.as_ptr())
        } else {
            false
        }
    }

    fn new(len: usize) -> Self {
        let ptr = if len == 0 {
            None
        } else {
            let layout = Layout::from_size_align(len, 8).unwrap();
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
            byte_length: len,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.byte_length
    }

    pub fn view_len<T: Viewable>(&self, byte_offset: usize) -> usize {
        let size = std::mem::size_of::<T>();
        (self.byte_length - byte_offset) / size
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

    pub fn get<T: Viewable>(&self, offset: usize) -> Option<T> {
        let size = std::mem::size_of::<T>();
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

    pub fn get_offset_by_byte<T: Viewable>(&self, byte_offset: usize) -> Option<T> {
        let size = std::mem::size_of::<T>();
        let end_byte_offset = byte_offset + size;
        if end_byte_offset > self.byte_length {
            None
        } else {
            self.ptr.map(|data| {
                // SAFETY: The data is properly initialized, and the T being read is
                // checked to be fully within the length of the data allocation.
                unsafe { read_unaligned(data.as_ptr().byte_add(byte_offset).cast()) }
            })
        }
    }

    pub fn set<T: Viewable>(&mut self, offset: usize, value: T) {
        let size = std::mem::size_of::<T>();
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

    pub fn set_offset_by_byte<T: Viewable>(&mut self, byte_offset: usize, value: T) {
        let size = std::mem::size_of::<T>();
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

    pub fn set_from<T: Viewable>(
        &mut self,
        dst_offset: usize,
        src: &DataBlock,
        src_offset: usize,
        count: usize,
    ) {
        let size = std::mem::size_of::<T>();
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

    pub fn copy_within<T: Viewable>(&mut self, dst_offset: usize, src_offset: usize, count: usize) {
        let size = std::mem::size_of::<T>();
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
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr, self.byte_length) };
            slice.copy_within(
                src_byte_offset..(src_byte_offset + byte_length),
                dst_byte_offset,
            );
        }
    }

    /// ### [6.2.9.1 CreateByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createbytedatablock)
    ///
    /// The abstract operation CreateByteDataBlock takes argument size (a
    /// non-negative integer) and returns either a normal completion containing
    /// a Data Block or a throw completion.
    pub fn create_byte_data_block(agent: &mut Agent, size: u64) -> JsResult<Self> {
        // 1. If size > 2**53 - 1, throw a RangeError exception.
        if size > u64::pow(2, 53) - 1 {
            // TODO: throw a RangeError exception
            Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Not a safe integer",
            ))
        } else if let Ok(size) = usize::try_from(size) {
            // 2. Let db be a new Data Block value consisting of size bytes.
            // 3. Set all of the bytes of db to 0.
            // 4. Return db.
            Ok(Self::new(size))
        } else {
            // 2. cont: If it is impossible to create such a Data Block, throw a RangeError exception.
            // TODO: throw a RangeError exception
            Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "Invalid Data Block length",
            ))
        }
    }

    /// ### [6.2.9.2 CreateSharedByteDataBlock ( size )](https://tc39.es/ecma262/#sec-createsharedbytedatablock)
    ///
    /// The abstract operation CreateSharedByteDataBlock takes argument size (a
    /// non-negative integer) and returns either a normal completion containing
    /// a Shared Data Block or a throw completion.
    pub fn create_shared_byte_data_block(agent: &mut Agent, size: u64) -> JsResult<Self> {
        // 1. Let db be a new Shared Data Block value consisting of size bytes. If it is impossible to create such a Shared Data Block, throw a RangeError exception.
        if let Ok(size) = usize::try_from(size) {
            // 2. Let execution be the [[CandidateExecution]] field of the surrounding agent's Agent Record.
            // 3. Let eventsRecord be the Agent Events Record of execution.[[EventsRecords]] whose [[AgentSignifier]] is AgentSignifier().
            // 4. Let zero be « 0 ».
            // 5. For each index i of db, do
            // a. Append WriteSharedMemory { [[Order]]: INIT, [[NoTear]]: true, [[Block]]: db, [[ByteIndex]]: i, [[ElementSize]]: 1, [[Payload]]: zero } to eventsRecord.[[EventList]].
            Ok(Self::new(size))
        } else {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Invalid Shared Data Block length",
            ))
        }
        // 6. Return db.
    }

    /// ### [6.2.9.3 CopyDataBlockBytes ( toBlock, toIndex, fromBlock, fromIndex, count )](https://tc39.es/ecma262/#sec-copydatablockbytes)
    ///
    /// The abstract operation CopyDataBlockBytes takes arguments toBlock (a
    /// Data Block or a Shared Data Block), toIndex (a non-negative integer),
    /// fromBlock (a Data Block or a Shared Data Block), fromIndex (a
    /// non-negative integer), and count (a non-negative integer) and returns
    /// UNUSED.
    pub fn copy_data_block_bytes(
        &mut self,
        to_index: usize,
        from_block: &Self,
        from_index: usize,
        count: usize,
    ) {
        let to_block = self;
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
        unsafe { to_ptr.copy_from_nonoverlapping(from_ptr, count) };
        // 7. Return UNUSED.
    }

    pub fn realloc(&mut self, new_byte_length: usize) {
        // Max byte length should be within safe integer length.
        debug_assert!(new_byte_length < 2usize.pow(53));
        let ptr = self
            .as_mut_ptr(0)
            .expect("Tried to realloc a detached DataBlock");
        let layout = Layout::from_size_align(self.byte_length, 8).unwrap();
        if new_byte_length == 0 {
            // When resizing to zero, we just drop the data instead.
            if let Some(ptr) = self.ptr {
                unsafe { dealloc(ptr.as_ptr(), layout) };
            }
            self.ptr = None;
            self.byte_length = 0;
            return;
        }
        // SAFETY: `ptr` can currently only come from GlobalAllocator, it was
        // allocated with `Layout::from_size_align(self.byte_length, 8)`, new
        // size is non-zero, and cannot overflow isize (on a 64-bit machine).
        let ptr = unsafe { realloc(ptr, layout, new_byte_length) };
        self.ptr = NonNull::new(ptr);
        self.byte_length = new_byte_length;
    }
}

#[test]
fn new_data_block() {
    let db = DataBlock::new(0);
    assert_eq!(db.len(), 0);
    assert_eq!(db.get::<u8>(0), None);

    let db = DataBlock::new(8);
    assert_eq!(db.len(), 8);
    for i in 0..8 {
        assert_eq!(db.get::<u8>(i), Some(0));
    }
}

#[test]
fn data_block_set() {
    let mut db = DataBlock::new(8);
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
    let mut db = DataBlock::new(8);
    let mut db2 = DataBlock::new(8);
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
    let mut db = DataBlock::new(8);
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
