use std::{
    alloc::{Layout, LayoutError},
    marker::PhantomData,
    ptr::NonNull,
};

pub trait SoAble: Sized {
    type TupleRepr: SoATuple;
    type RefTuple<'a>: Copy
    where
        Self: 'a;
    type MutTuple<'a>
    where
        Self: 'a;
    type SliceTuple<'a>: Copy
    where
        Self: 'a;
    type SliceMutTuple<'a>
    where
        Self: 'a;

    fn into_tuple(value: Self) -> Self::TupleRepr;
    fn from_tuple(value: Self::TupleRepr) -> Self;
    fn as_ref<'a>(
        _: PhantomData<&'a Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
    ) -> Self::RefTuple<'a>;
    fn as_mut<'a>(
        _: PhantomData<&'a mut Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
    ) -> Self::MutTuple<'a>;
    fn as_slice<'a>(
        _: PhantomData<&'a Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
        len: u32,
    ) -> Self::SliceTuple<'a>;
    fn as_mut_slice<'a>(
        _: PhantomData<&'a mut Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
        len: u32,
    ) -> Self::SliceMutTuple<'a>;
}

pub trait SoATuple {
    type Offsets: Copy;
    type Pointers: Copy;

    fn layout(capacity: u32) -> Result<Layout, LayoutError>;

    fn get_offsets(capacity: u32) -> Self::Offsets;

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32, len: u32);

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32);

    unsafe fn get_pointers(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self::Pointers;
}

impl<T, U> SoATuple for (T, U) {
    type Offsets = usize;
    type Pointers = (NonNull<T>, NonNull<U>);

    fn layout(capacity: u32) -> Result<Layout, LayoutError> {
        Ok(layout_array::<T>(capacity)?
            .extend(layout_array::<U>(capacity)?)?
            .0
            .pad_to_align())
    }

    fn get_offsets(capacity: u32) -> usize {
        // SAFETY: method is guaranteed to call with checked capacities.
        unsafe {
            let layout = layout_array::<T>(capacity).unwrap_unchecked();
            let (_, u_offset) = extend_layout_array::<U>(layout, capacity).unwrap_unchecked();
            u_offset
        }
    }

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        debug_assert!(ptr.cast::<Self>().is_aligned());
        debug_assert!(old_capacity < new_capacity);
        let old_u_offset = Self::get_offsets(old_capacity);
        let new_u_offset = Self::get_offsets(new_capacity);

        let old_u_ptr = ptr.byte_add(old_u_offset).cast::<U>();
        debug_assert!(old_u_ptr.is_aligned());

        let new_u_ptr = ptr.byte_add(new_u_offset).cast::<U>();
        debug_assert!(new_u_ptr.is_aligned());

        // SAFETY: old data is located at old_u_ptr and its length is len
        unsafe {
            // Copy old data to new allocation area.
            core::ptr::copy(old_u_ptr.as_ptr(), new_u_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_u_ptr.write_bytes(0, len as usize);
        };
    }

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32) {
        if size_of::<Self>() == 0 {
            return;
        }
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let u_offset = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(len as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(len as usize);
        debug_assert!(u_ptr.is_aligned());

        t_ptr.write(data.0);
        u_ptr.write(data.1);
    }

    unsafe fn get_pointers(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self::Pointers {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let u_offset = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(index as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(index as usize);
        debug_assert!(u_ptr.is_aligned());

        (t_ptr, u_ptr)
    }
}

impl<T, U, V> SoATuple for (T, U, V) {
    type Offsets = (usize, usize);
    type Pointers = (NonNull<T>, NonNull<U>, NonNull<V>);

    fn layout(capacity: u32) -> Result<Layout, LayoutError> {
        Ok(layout_array::<T>(capacity)?
            .extend(layout_array::<U>(capacity)?)?
            .0
            .extend(layout_array::<V>(capacity)?)?
            .0
            .pad_to_align())
    }

    fn get_offsets(capacity: u32) -> Self::Offsets {
        // SAFETY: method is guaranteed to call with checked capacities.
        unsafe {
            let layout = layout_array::<T>(capacity).unwrap_unchecked();
            let (layout, u_offset) = extend_layout_array::<U>(layout, capacity).unwrap_unchecked();
            let (_, v_offset) = extend_layout_array::<V>(layout, capacity).unwrap_unchecked();
            (u_offset, v_offset)
        }
    }

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        debug_assert!(ptr.cast::<Self>().is_aligned());
        debug_assert!(old_capacity < new_capacity);
        // SAFETY: old allocation; the layout has been checked.
        let (old_u_offset, old_v_offset) = Self::get_offsets(old_capacity);
        let (new_u_offset, new_v_offset) = Self::get_offsets(new_capacity);

        let old_u_ptr = ptr.byte_add(old_u_offset).cast::<U>();
        debug_assert!(old_u_ptr.is_aligned());
        let old_v_ptr = ptr.byte_add(old_v_offset).cast::<V>();
        debug_assert!(old_v_ptr.is_aligned());

        let new_u_ptr = ptr.byte_add(new_u_offset).cast::<U>();
        debug_assert!(new_u_ptr.is_aligned());
        let new_v_ptr = ptr.byte_add(new_v_offset).cast::<V>();
        debug_assert!(new_v_ptr.is_aligned());

        // SAFETY: old data is located at old_*_ptr and its length is len
        unsafe {
            // Copy old data to new allocation area.
            core::ptr::copy(old_v_ptr.as_ptr(), new_v_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_v_ptr.write_bytes(0, len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(old_u_ptr.as_ptr(), new_u_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_u_ptr.write_bytes(0, len as usize);
        };
    }

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32) {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let (u_offset, v_offset) = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(len as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(len as usize);
        debug_assert!(u_ptr.is_aligned());
        let v_ptr = ptr.byte_add(v_offset).cast::<V>().add(len as usize);
        debug_assert!(v_ptr.is_aligned());

        t_ptr.write(data.0);
        u_ptr.write(data.1);
        v_ptr.write(data.2);
    }

    unsafe fn get_pointers(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self::Pointers {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let (u_offset, v_offset) = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(index as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(index as usize);
        debug_assert!(u_ptr.is_aligned());
        let v_ptr = ptr.byte_add(v_offset).cast::<V>().add(index as usize);
        debug_assert!(v_ptr.is_aligned());

        debug_assert!(u_ptr.is_aligned());
        (t_ptr, u_ptr, v_ptr)
    }
}

impl<T, U, V, W> SoATuple for (T, U, V, W) {
    type Offsets = (usize, usize, usize);
    type Pointers = (NonNull<T>, NonNull<U>, NonNull<V>, NonNull<W>);

    fn layout(capacity: u32) -> Result<Layout, LayoutError> {
        Ok(layout_array::<T>(capacity)?
            .extend(layout_array::<U>(capacity)?)?
            .0
            .extend(layout_array::<V>(capacity)?)?
            .0
            .extend(layout_array::<W>(capacity)?)?
            .0
            .pad_to_align())
    }

    fn get_offsets(capacity: u32) -> Self::Offsets {
        // SAFETY: method is guaranteed to call with checked capacities.
        unsafe {
            let layout = layout_array::<T>(capacity).unwrap_unchecked();
            let (layout, u_offset) = extend_layout_array::<U>(layout, capacity).unwrap_unchecked();
            let (layout, v_offset) = extend_layout_array::<V>(layout, capacity).unwrap_unchecked();
            let (_, w_offset) = extend_layout_array::<W>(layout, capacity).unwrap_unchecked();
            (u_offset, v_offset, w_offset)
        }
    }

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        debug_assert!(ptr.cast::<Self>().is_aligned());
        debug_assert!(old_capacity < new_capacity);
        // SAFETY: old allocation; the layout has been checked.
        let (old_u_offset, old_v_offset, old_w_offset) = Self::get_offsets(old_capacity);
        let (new_u_offset, new_v_offset, new_w_offset) = Self::get_offsets(new_capacity);

        let old_u_ptr = ptr.byte_add(old_u_offset).cast::<U>();
        debug_assert!(old_u_ptr.is_aligned());
        let old_v_ptr = ptr.byte_add(old_v_offset).cast::<V>();
        debug_assert!(old_v_ptr.is_aligned());
        let old_w_ptr = ptr.byte_add(old_w_offset).cast::<W>();
        debug_assert!(old_w_ptr.is_aligned());

        let new_u_ptr = ptr.byte_add(new_u_offset).cast::<U>();
        debug_assert!(new_u_ptr.is_aligned());
        let new_v_ptr = ptr.byte_add(new_v_offset).cast::<V>();
        debug_assert!(new_v_ptr.is_aligned());
        let new_w_ptr = ptr.byte_add(new_w_offset).cast::<W>();
        debug_assert!(new_w_ptr.is_aligned());

        // SAFETY: old data is located at old_*_ptr and its length is len
        unsafe {
            // Copy old data to new allocation area.
            core::ptr::copy(old_w_ptr.as_ptr(), new_w_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_w_ptr.write_bytes(0, len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(old_v_ptr.as_ptr(), new_v_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_v_ptr.write_bytes(0, len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(old_u_ptr.as_ptr(), new_u_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_u_ptr.write_bytes(0, len as usize);
        };
    }

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32) {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let (u_offset, v_offset, w_offset) = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(len as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(len as usize);
        debug_assert!(u_ptr.is_aligned());
        let v_ptr = ptr.byte_add(v_offset).cast::<V>().add(len as usize);
        debug_assert!(v_ptr.is_aligned());
        let w_ptr = ptr.byte_add(w_offset).cast::<W>().add(len as usize);
        debug_assert!(w_ptr.is_aligned());

        t_ptr.write(data.0);
        u_ptr.write(data.1);
        v_ptr.write(data.2);
        w_ptr.write(data.3);
    }

    unsafe fn get_pointers(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self::Pointers {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let (u_offset, v_offset, w_offset) = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(index as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(index as usize);
        debug_assert!(u_ptr.is_aligned());
        let v_ptr = ptr.byte_add(v_offset).cast::<V>().add(index as usize);
        debug_assert!(v_ptr.is_aligned());
        let w_ptr = ptr.byte_add(w_offset).cast::<W>().add(index as usize);
        debug_assert!(w_ptr.is_aligned());

        (t_ptr, u_ptr, v_ptr, w_ptr)
    }
}

impl<T, U, V, W, X> SoATuple for (T, U, V, W, X) {
    type Offsets = (usize, usize, usize, usize);
    type Pointers = (NonNull<T>, NonNull<U>, NonNull<V>, NonNull<W>, NonNull<X>);

    fn layout(capacity: u32) -> Result<Layout, LayoutError> {
        Ok(layout_array::<T>(capacity)?
            .extend(layout_array::<U>(capacity)?)?
            .0
            .extend(layout_array::<V>(capacity)?)?
            .0
            .extend(layout_array::<W>(capacity)?)?
            .0
            .extend(layout_array::<X>(capacity)?)?
            .0
            .pad_to_align())
    }

    fn get_offsets(capacity: u32) -> Self::Offsets {
        // SAFETY: method is guaranteed to call with checked capacities.
        unsafe {
            let layout = layout_array::<T>(capacity).unwrap_unchecked();
            let (layout, u_offset) = extend_layout_array::<U>(layout, capacity).unwrap_unchecked();
            let (layout, v_offset) = extend_layout_array::<V>(layout, capacity).unwrap_unchecked();
            let (layout, w_offset) = extend_layout_array::<W>(layout, capacity).unwrap_unchecked();
            let (_, x_offset) = extend_layout_array::<X>(layout, capacity).unwrap_unchecked();
            (u_offset, v_offset, w_offset, x_offset)
        }
    }

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        debug_assert!(ptr.cast::<Self>().is_aligned());
        debug_assert!(old_capacity < new_capacity);
        // SAFETY: old allocation; the layout has been checked.
        let (old_u_offset, old_v_offset, old_w_offset, old_x_offset) =
            Self::get_offsets(old_capacity);
        let (new_u_offset, new_v_offset, new_w_offset, new_x_offset) =
            Self::get_offsets(new_capacity);

        let old_u_ptr = ptr.byte_add(old_u_offset).cast::<U>();
        debug_assert!(old_u_ptr.is_aligned());
        let old_v_ptr = ptr.byte_add(old_v_offset).cast::<V>();
        debug_assert!(old_v_ptr.is_aligned());
        let old_w_ptr = ptr.byte_add(old_w_offset).cast::<W>();
        debug_assert!(old_w_ptr.is_aligned());
        let old_x_ptr = ptr.byte_add(old_x_offset).cast::<X>();
        debug_assert!(old_x_ptr.is_aligned());

        let new_u_ptr = ptr.byte_add(new_u_offset).cast::<U>();
        debug_assert!(new_u_ptr.is_aligned());
        let new_v_ptr = ptr.byte_add(new_v_offset).cast::<V>();
        debug_assert!(new_v_ptr.is_aligned());
        let new_w_ptr = ptr.byte_add(new_w_offset).cast::<W>();
        debug_assert!(new_w_ptr.is_aligned());
        let new_x_ptr = ptr.byte_add(new_x_offset).cast::<X>();
        debug_assert!(new_x_ptr.is_aligned());

        // SAFETY: old data is located at old_*_ptr and its length is len
        unsafe {
            // Copy old data to new allocation area.
            core::ptr::copy(old_x_ptr.as_ptr(), new_x_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_x_ptr.write_bytes(0, len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(old_w_ptr.as_ptr(), new_w_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_w_ptr.write_bytes(0, len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(old_v_ptr.as_ptr(), new_v_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_v_ptr.write_bytes(0, len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(old_u_ptr.as_ptr(), new_u_ptr.as_ptr(), len as usize);
            // Zero out the old data.
            old_u_ptr.write_bytes(0, len as usize);
        };
    }

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32) {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let (u_offset, v_offset, w_offset, x_offset) = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(len as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(len as usize);
        debug_assert!(u_ptr.is_aligned());
        let v_ptr = ptr.byte_add(v_offset).cast::<V>().add(len as usize);
        debug_assert!(v_ptr.is_aligned());
        let w_ptr = ptr.byte_add(w_offset).cast::<W>().add(len as usize);
        debug_assert!(w_ptr.is_aligned());
        let x_ptr = ptr.byte_add(x_offset).cast::<X>().add(len as usize);
        debug_assert!(x_ptr.is_aligned());

        t_ptr.write(data.0);
        u_ptr.write(data.1);
        v_ptr.write(data.2);
        w_ptr.write(data.3);
        x_ptr.write(data.4);
    }

    unsafe fn get_pointers(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self::Pointers {
        debug_assert!(ptr.cast::<Self>().is_aligned());
        let (u_offset, v_offset, w_offset, x_offset) = Self::get_offsets(capacity);

        let t_ptr = ptr.cast::<T>().add(index as usize);
        debug_assert!(t_ptr.is_aligned());
        let u_ptr = ptr.byte_add(u_offset).cast::<U>().add(index as usize);
        debug_assert!(u_ptr.is_aligned());
        let v_ptr = ptr.byte_add(v_offset).cast::<V>().add(index as usize);
        debug_assert!(v_ptr.is_aligned());
        let w_ptr = ptr.byte_add(w_offset).cast::<W>().add(index as usize);
        debug_assert!(w_ptr.is_aligned());
        let x_ptr = ptr.byte_add(x_offset).cast::<X>().add(index as usize);
        debug_assert!(x_ptr.is_aligned());

        (t_ptr, u_ptr, v_ptr, w_ptr, x_ptr)
    }
}

#[inline]
fn extend_layout_array<T>(layout: Layout, cap: u32) -> Result<(Layout, usize), LayoutError> {
    layout.extend(layout_array::<T>(cap)?)
}

#[inline]
fn layout_array<T>(cap: u32) -> Result<Layout, LayoutError> {
    let elem_layout = Layout::new::<T>();
    Layout::from_size_align(elem_layout.size() * cap as usize, elem_layout.align())
}

#[macro_export]
macro_rules! soable {
    ($target:ident) => (
        compile_error!("soable macro requires explicit field names and types")
    );
    ($target:ident { $field:ident: $type:ty }) => (
        compile_error!("Single-field structs not supported; use a normal Vec")
    );
    ($target:ident { $($field:ident: $type:ty),+ }) => {
        impl SoAble for $target {
            type TupleRepr = ($($type),+);
            type RefTuple<'a> = ($(&'a $type),+);
            type MutTuple<'a> = ($(&'a mut $type),+);
            type SliceTuple<'a> = ($(&'a [$type]),+);
            type SliceMutTuple<'a> = ($(&'a mut [$type]),+);

            fn into_tuple(value: Self) -> Self::TupleRepr {
                let Self { $($field),+ } = value;
                ($($field),+)
            }

            fn from_tuple(value: Self::TupleRepr) -> Self {
                let ($($field),+) = value;
                Self { $($field),+ }
            }

            fn as_ref<'a>(
                _: PhantomData<&'a Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
            ) -> Self::RefTuple<'a> {
                let ($($field),+) = value;
                unsafe {
                    ($($field.as_ref()),+)
                }
            }

            fn as_mut<'a>(
                _: PhantomData<&'a mut Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
            ) -> Self::MutTuple<'a> {
                let ($(mut $field),+) = value;
                unsafe {
                    ($($field.as_mut()),+)
                }
            }

            fn as_slice<'a>(
                _: PhantomData<&'a Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
                len: u32,
            ) -> Self::SliceTuple<'a> {
                let len = len as usize;
                let ($($field),+) = value;
                unsafe {
                    (
                        $(core::slice::from_raw_parts($field.as_ptr(), len)),+
                    )
                }
            }

            fn as_mut_slice<'a>(
                _: PhantomData<&'a mut Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
                len: u32,
            ) -> Self::SliceMutTuple<'a> {
                let len = len as usize;
                let ($($field),+) = value;
                unsafe {
                    (
                        $(core::slice::from_raw_parts_mut($field.as_ptr(), len)),+
                    )
                }
            }
        }
    };
}
