use std::{
    alloc::{Layout, LayoutError},
    ptr::NonNull,
};

pub trait SoAble {
    type TupleRepr: SoATuple;

    fn to_tuple(value: Self) -> Self::TupleRepr;
    fn from_tuple(value: Self::TupleRepr) -> Self;
}

pub trait SoATuple {
    fn layout(capacity: u32) -> Result<Layout, LayoutError>;

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32);

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32);

    unsafe fn get_cloned(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self;
}

impl<T, U> SoATuple for (T, U) {
    fn layout(capacity: u32) -> Result<Layout, LayoutError> {
        Ok(layout_array(capacity, Layout::new::<T>())?
            .extend(layout_array(capacity, Layout::new::<U>())?)?
            .0
            .pad_to_align())
    }

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32) {
        if old_capacity == 0 {
            return;
        }
        debug_assert!(ptr.cast::<Self>().is_aligned());
        debug_assert!(old_capacity < new_capacity);
        // SAFETY: old allocation; the layout has been checked.
        let old_t_layout_aligned = unsafe {
            layout_array(old_capacity, Layout::new::<T>())
                .unwrap_unchecked()
                .align_to(core::mem::align_of::<U>())
                .unwrap_unchecked()
        };
        // SAFETY: new allocation; the layout has been checked.
        let new_t_layout_aligned = unsafe {
            layout_array(new_capacity, Layout::new::<T>())
                .unwrap_unchecked()
                .align_to(core::mem::align_of::<U>())
                .unwrap_unchecked()
        };
        let old_u_ptr = ptr
            .byte_offset(old_t_layout_aligned.size() as isize)
            .cast::<U>();
        debug_assert!(old_u_ptr.is_aligned());
        let new_u_ptr = ptr
            .byte_offset(new_t_layout_aligned.size() as isize)
            .cast::<U>();
        debug_assert!(new_u_ptr.is_aligned());
        // SAFETY: old data is located at old_u_ptr and its length is old_capacity
        unsafe {
            // Write old data to new allocation area.
            core::ptr::copy(
                old_u_ptr.as_ptr(),
                new_u_ptr.as_ptr(),
                old_capacity as usize,
            );
            // Zero out the old data.
            old_u_ptr.write_bytes(0, old_capacity as usize);
        };
    }

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32) {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        // SAFETY: old allocation; the layout has been checked.
        let t_layout_aligned = unsafe {
            layout_array(capacity, Layout::new::<T>())
                .unwrap_unchecked()
                .align_to(core::mem::align_of::<U>())
                .unwrap_unchecked()
        };
        let t_ptr = ptr.cast::<T>().offset(len as isize);

        let u_ptr = ptr
            .byte_offset(t_layout_aligned.size() as isize)
            .cast::<U>()
            .offset(len as isize);
        debug_assert!(u_ptr.is_aligned());
        t_ptr.write(data.0);
        u_ptr.write(data.1);
    }

    unsafe fn get_cloned(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        // SAFETY: old allocation; the layout has been checked.
        let t_layout_aligned = unsafe {
            layout_array(capacity, Layout::new::<T>())
                .unwrap_unchecked()
                .align_to(core::mem::align_of::<U>())
                .unwrap_unchecked()
        };
        let t_ptr = ptr.cast::<T>().offset(index as isize);

        let u_ptr = ptr
            .byte_offset(t_layout_aligned.size() as isize)
            .cast::<U>()
            .offset(index as isize);
        debug_assert!(u_ptr.is_aligned());
        (t_ptr.read(), u_ptr.read())
    }
}

#[inline]
fn layout_array(cap: u32, elem_layout: Layout) -> Result<Layout, LayoutError> {
    Layout::from_size_align(elem_layout.size() * cap as usize, elem_layout.align())
}
