use super::{
    function::FunctionHeapData, heap_constants::WellKnownSymbolIndexes, indexes::ObjectIndex,
    object::ObjectEntry,
};
use crate::{
    ecmascript::types::{Object, PropertyKey, Value},
    heap::{
        heap_constants::{get_constructor_index, BuiltinObjectIndexes},
        Heap, PropertyDescriptor,
    },
};
use std::{
    alloc::{alloc_zeroed, handle_alloc_error, Layout},
    ptr::{read_unaligned, write_bytes, write_unaligned, NonNull},
};

/// Backing store for ArrayBuffers
///
/// The pointer points to a continuous buffer
/// of bytes, the length of which is determined by
/// the capacity or length. The pointer can be None
/// if the length of the buffer is zero.
#[derive(Debug, Copy, Clone)]
pub struct BackingStore {
    ptr: Option<NonNull<u8>>,
    cap: u32,
    byte_length: u32,
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

impl BackingStore {
    pub fn new(len: u32) -> Self {
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
        src: &BackingStore,
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
}

#[derive(Debug)]
pub enum InternalBuffer {
    Detached,
    Growable(BackingStore),
    Static(BackingStore),
    // Shared(SharedBackingStoreIndex)
}

#[derive(Debug)]
pub struct ArrayBufferHeapData {
    pub(super) object_index: Option<ObjectIndex>,
    pub(super) buffer: InternalBuffer,
    // detach_key
}

pub fn initialize_array_buffer_heap(heap: &mut Heap) {
    let species_function_name = Value::from_str(heap, "get [Symbol.species]");
    let byte_length_key = Value::from_str(heap, "get byteLength");
    let entries = vec![
        ObjectEntry::new_prototype_function_entry(heap, "isView", 1, false),
        ObjectEntry::new_constructor_prototype_entry(
            heap,
            BuiltinObjectIndexes::ArrayBufferPrototypeIndex.into(),
        ),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::Species.into()),
            PropertyDescriptor::ReadOnly {
                get: heap.create_function(species_function_name, 0, false),
                enumerable: false,
                configurable: true,
            },
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ArrayBufferConstructorIndex,
        true,
        Some(Object::Function(
            BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
        )),
        entries,
    );
    heap.functions
        [get_constructor_index(BuiltinObjectIndexes::ArrayBufferConstructorIndex).into_index()] =
        Some(FunctionHeapData {
            object_index: Some(BuiltinObjectIndexes::ArrayBufferConstructorIndex.into()),
            length: 1,
            // bound: None,
            // visible: None,
            // binding: array_buffer_constructor_binding,
            initial_name: Value::Null,
        });
    let entries = vec![
        ObjectEntry::new(
            PropertyKey::from_str(heap, "byteLength"),
            PropertyDescriptor::ReadOnly {
                get: heap.create_function(byte_length_key, 0, false),
                enumerable: false,
                configurable: true,
            },
        ),
        ObjectEntry::new(
            PropertyKey::from_str(heap, "constructor"),
            PropertyDescriptor::rwx(Value::Function(get_constructor_index(
                BuiltinObjectIndexes::ArrayBufferConstructorIndex,
            ))),
        ),
        ObjectEntry::new_prototype_function_entry(heap, "slice", 2, false),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag.into()),
            PropertyDescriptor::roxh(Value::from_str(heap, "ArrayBuffer")),
        ),
    ];
    heap.insert_builtin_object(
        BuiltinObjectIndexes::ArrayBufferPrototypeIndex,
        true,
        Some(Object::Object(
            BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
        )),
        entries,
    );
}

#[test]
fn new_backing_store() {
    let bs = BackingStore::new(0);
    assert_eq!(bs.len(), 0);
    assert_eq!(bs.capacity(), 0);
    assert_eq!(bs.get::<u8>(0), None);

    let bs = BackingStore::new(8);
    assert_eq!(bs.len(), 8);
    assert_eq!(bs.capacity(), 8);
    for i in 0..8 {
        assert_eq!(bs.get::<u8>(i as u32), Some(0));
    }
}

#[test]
fn new_backing_store_with_capacity() {
    let bs = BackingStore::new_with_capacity(0, 8);
    assert_eq!(bs.len(), 0);
    assert_eq!(bs.capacity(), 8);
    for i in 0..8 {
        assert_eq!(bs.get::<u8>(i as u32), None);
    }

    let bs = BackingStore::new_with_capacity(8, 16);
    assert_eq!(bs.len(), 8);
    assert_eq!(bs.capacity(), 16);
    for i in 0..8 {
        assert_eq!(bs.get::<u8>(i as u32), Some(0));
    }
    for i in 8..16 {
        assert_eq!(bs.get::<u8>(i as u32), None);
    }
}

#[test]
fn backing_store_set() {
    let mut bs = BackingStore::new(8);
    assert_eq!(bs.len(), 8);
    assert_eq!(bs.capacity(), 8);
    for i in 0..8 {
        assert_eq!(bs.get::<u8>(i as u32), Some(0));
    }

    for i in 0..8 {
        bs.set::<u8>(i as u32, i + 1);
    }

    for i in 0..8 {
        assert_eq!(bs.get::<u8>(i as u32), Some(i + 1));
    }
}

#[test]
fn backing_store_resize() {
    let mut bs = BackingStore::new_with_capacity(0, 8);
    bs.resize(8);
    assert_eq!(bs.len(), 8);
    assert_eq!(bs.capacity(), 8);
    for i in 0..8 {
        assert_eq!(bs.get::<u8>(i as u32), Some(0));
    }

    for i in 0..8 {
        bs.set::<u8>(i as u32, i + 1);
    }

    let ptr = bs.as_ptr(0).unwrap();
    bs.resize(0);

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
fn backing_store_set_from() {
    let mut bs = BackingStore::new(8);
    let mut bs2 = BackingStore::new(8);
    for i in 0..8 {
        assert_eq!(bs.get::<u8>(0), Some(0));
        bs2.set::<u8>(i as u32, i + 1);
    }
    assert_eq!(bs2.get::<u8>(0), Some(1));
    assert_eq!(bs2.get::<u8>(1), Some(2));
    assert_eq!(bs2.get::<u8>(2), Some(3));
    assert_eq!(bs2.get::<u8>(3), Some(4));
    assert_eq!(bs2.get::<u8>(4), Some(5));
    assert_eq!(bs2.get::<u8>(5), Some(6));
    assert_eq!(bs2.get::<u8>(6), Some(7));
    assert_eq!(bs2.get::<u8>(7), Some(8));
    bs.set_from::<u8>(0, &bs2, 4, 4);
    assert_eq!(bs.get::<u8>(0), Some(5));
    assert_eq!(bs.get::<u8>(1), Some(6));
    assert_eq!(bs.get::<u8>(2), Some(7));
    assert_eq!(bs.get::<u8>(3), Some(8));
    assert_eq!(bs.get::<u8>(4), Some(0));
    assert_eq!(bs.get::<u8>(5), Some(0));
    assert_eq!(bs.get::<u8>(6), Some(0));
    assert_eq!(bs.get::<u8>(7), Some(0));

    // Reset
    for i in 0..8 {
        bs.set::<u8>(i as u32, i + 1);
    }
    bs.copy_within::<u8>(2, 4, 4);
    assert_eq!(bs.get::<u8>(0), Some(1));
    assert_eq!(bs.get::<u8>(1), Some(2));
    assert_eq!(bs.get::<u8>(2), Some(5));
    assert_eq!(bs.get::<u8>(3), Some(6));
    assert_eq!(bs.get::<u8>(4), Some(7));
    assert_eq!(bs.get::<u8>(5), Some(8));
    assert_eq!(bs.get::<u8>(6), Some(7));
    assert_eq!(bs.get::<u8>(7), Some(8));
}

#[test]
fn backing_store_copy_within() {
    let mut bs = BackingStore::new(8);
    for i in 0..8 {
        bs.set::<u8>(i as u32, i + 1);
    }
    assert_eq!(bs.get::<u8>(0), Some(1));
    assert_eq!(bs.get::<u8>(1), Some(2));
    assert_eq!(bs.get::<u8>(2), Some(3));
    assert_eq!(bs.get::<u8>(3), Some(4));
    assert_eq!(bs.get::<u8>(4), Some(5));
    assert_eq!(bs.get::<u8>(5), Some(6));
    assert_eq!(bs.get::<u8>(6), Some(7));
    assert_eq!(bs.get::<u8>(7), Some(8));
    bs.copy_within::<u8>(0, 4, 4);
    assert_eq!(bs.get::<u8>(0), Some(5));
    assert_eq!(bs.get::<u8>(1), Some(6));
    assert_eq!(bs.get::<u8>(2), Some(7));
    assert_eq!(bs.get::<u8>(3), Some(8));
    assert_eq!(bs.get::<u8>(4), Some(5));
    assert_eq!(bs.get::<u8>(5), Some(6));
    assert_eq!(bs.get::<u8>(6), Some(7));
    assert_eq!(bs.get::<u8>(7), Some(8));

    // Reset
    for i in 0..8 {
        bs.set::<u8>(i as u32, i + 1);
    }
    bs.copy_within::<u8>(2, 4, 4);
    assert_eq!(bs.get::<u8>(0), Some(1));
    assert_eq!(bs.get::<u8>(1), Some(2));
    assert_eq!(bs.get::<u8>(2), Some(5));
    assert_eq!(bs.get::<u8>(3), Some(6));
    assert_eq!(bs.get::<u8>(4), Some(7));
    assert_eq!(bs.get::<u8>(5), Some(8));
    assert_eq!(bs.get::<u8>(6), Some(7));
    assert_eq!(bs.get::<u8>(7), Some(8));
}
