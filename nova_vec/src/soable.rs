use std::{
    alloc::{Layout, LayoutError},
    marker::PhantomData,
    ptr::{drop_in_place, NonNull},
};

/// Used for defining the format and API of a type stored in `SoAVec` in a
/// Struct-of-Arrays format.
///
/// This trait defines a representation for the implementing type that the
/// `SoAVec` recognises and knows how to store, and the conversions to and from
/// said type. Additionally, the trait defines the necessary references types
/// for exposing the type's data when borrowed from the `SoAVec`.
///
/// For simple structs that are only a collection of individual fields, the
/// `soable!` macro can be used to easily map the fields into an equivalent
/// tuple representation. For more involved types such as structs with safety
/// invariants, unions, or enums the trait should be implemented manually with
/// all the necessary safety requirements considered.
///
/// # Safety requirements
///
/// 1. The type must be safely droppable field-wise, **or** the
///    `MUST_DROP_AS_SELF` boolean must be set. If it is set, `SoAVec`
///    guarantees that each dropped entry in the Struct-of-Arrays is read out
///    onto the stack and dropped as `Self`.
/// 2. The type's internal invariants must be upheld by the `SoAble::Ref`,
///    `SoAble::Mut`, `SoAble::Slice`, and `SoAble::SliceMut` types.
///    Specifically, this means that if mutating a given field individually
///    could break invariants, then that field's (mutable) reference must not
///    be exposed by any of the SoAble reference types.
///
/// # When to manually implement `SoAble`
///
/// If your type is an enum, a union, or a struct with internal invariants then
/// a manual implementation of `SoAble` is required. A direct tuple
/// representation of any of these would allow internal invariants to be
/// broken through the SoAble reference types.
///
/// ## `SoAble` enums
///
/// For enums the Struct-of-Arrays format is likely to be
/// `(Discriminant, union Payload)`; the `Discimrinant` can be extracted
/// directly using `std::mem::discriminant` but the payload needs pattern
/// matching to extract and place inside the payload union. It is safe to
/// expose the payload union directly through the SoAble reference types
/// because accessing its data is unsafe (as a union); thus the implementation
/// of `SoAble` can be fairly straight-forward. Alternatively, a safe API
/// can be implemented to abstract over the discriminant-payload reference
/// pair.
///
/// ## `SoAble` unions
///
/// The representation is entirely up to and decidable by the implementor. If
/// you're thinking about this, then there's a good chance you should rather
/// move some shared data out of the union.
///
/// ## `SoAble` structs with internal invariants
///
/// A struct with internal invariants must manually implement `SoAble` such
/// that the exposed SoAble reference types cannot violate those internal
/// invariants.
///
/// ## Types with custom `Drop`
///
/// Any type that has a custom `Drop` needs to manually implement `SoAble`. The
/// conversion between `Self` and `Self::TupleRepr` should then move the `Self`
/// into `ManuallyDrop`, use exclusive references to the individual fields to
/// `std::ptr::read` the values out of `Self` and move the results into
/// `Self::TupleRepr`. Finally, let the `ManuallyDrop<Self>` to go out of scope
/// without dropping its contents.
///
/// Use `NEEDS_DROP` to indiciate if `Self` needs to be reconstructed for
/// dropping purposes.
///
/// # Fallibility
///
/// **This trait's methods should never unexpectedly fail**. Failure can be
/// extremely confusing. In the majority of uses it should be infallible,
/// though it may be acceptable to panic if the type or methods is misused
/// through programmer error, for example.
///
/// However, infallibility is not enforced and therefore not guaranteed.
/// As such, `unsafe` code should not rely on infallibility in general for
/// soundness.
///
/// # Examples
pub unsafe trait SoAble: Sized {
    /// Representation of the SoAble type in a Struct-of-Arrays friendly tuple
    /// form.
    ///
    /// The tuple does not need to strictly follow the field split or ordering
    /// of the original type, though that is generally a good starting point.
    ///
    /// The tuple form is identified by the SoATuple trait which is a sealed
    /// trait implemented by the crate for a select group of generic tuples.
    /// The form is thus required to match one of these presets.
    type TupleRepr: SoATuple;

    /// Set to true if the type must read out of the `SoAVec` and dropped as
    /// `Self` when deallocating.
    ///
    /// If the type's fields can be dropped directly in the Struct-of-Arrays
    /// format then this value should be false.
    ///
    /// # Examples
    ///
    /// A simple struct containing fields that required drop themselves but are
    /// not indvidually split up in the Struct-of-Arrays format can be dropped
    /// directly in the Struct-of-Arrays format.
    ///
    /// ```rust,ignore
    /// struct Simple {
    ///   a: Vec<u32>,
    ///   b: Box<u64>,
    /// }
    /// soable!(Simple { a: Vec<u32>, b: Box<u64> });
    /// ```
    ///
    /// A struct whose fields are not individually droppable must be read out
    /// of the `SoAVec` and dropped as `Self`.
    ///
    /// ```rust,ignore
    /// struct Complex {
    ///   ptr: NonNull<u32>,
    ///   len: u32,
    ///   cap: u32,
    /// }
    ///
    /// impl Drop for Complex {
    ///   fn drop(&mut self) {
    ///     // Note: deallocation requires access to ptr and cap.
    ///     core::mem::deallocate(self.ptr, array_layout(self.cap, Layout::new::<u32>()));
    ///   }
    /// }
    ///
    /// impl Soable for Complex {
    ///   const MUST_DROP_AS_SELF: bool = true;
    /// }
    /// ```
    ///
    /// f.ex. a field containing a `Vec` can be dropped in the Struct-of-Arrays
    /// format while a `Vec` split into two or three arrays would need to be
    /// read out and dropped as `Vec`.
    const MUST_DROP_AS_SELF: bool = false;

    /// Representation of the SoAble type as a group of references borrowed
    /// from the Struct-of-Arrays.
    ///
    /// Generally this will be a tuple of references matching the TupleRepr but
    /// in cases of types that split apart fields that have interconnected
    /// safety requirements that could be violated using shared references to
    /// individual fields, this type may be chosen to expose a safe interface
    /// over the group of field references.
    ///
    /// # Examples
    ///
    /// If a hypothetical `AtomicVec` was placed inside a `SoAVec` and shared
    /// references to its fields were exposed, then the `SoAVec` API would
    /// allow direct access to the `len` and `cap` fields that could be then
    /// used to mutate those without corresponding changes to `ptr`.
    ///
    /// In this case, the `AtomicVec` should use a different `Ref` type that
    /// does not allow such mutations to occur.
    ///
    /// ```rust,ignore
    /// struct AtomicVecSoaRef<'a, T> {
    ///   ptr: &'a AtomicPtr<T>,
    ///   cap: &'a AtomicUsize,
    ///   len: &'a AtomicUsize,
    /// }
    ///
    /// impl<T> SoAble for AtomicVec<T> {
    ///   type Ref<'a> = AtomicVecSoARef<'a, T> where Self: 'a;
    /// }
    /// ```
    type Ref<'a>: Copy
    where
        Self: 'a;

    /// Representation of the SoAble type as a group of exclusive references
    /// borrowed from the Struct-of-Arrays.
    ///
    /// Generally this will be a tuple of exclusive references matching the
    /// TupleRepr but in cases of types that split apart fields that have
    /// interconnected safety requirements that could be violated using
    /// exclusive references to individual fields, this type may be chosen to
    /// expose a safe interface over the group of exclusive field references.
    ///
    /// # Examples
    ///
    /// If a `Vec` was placed inside a `SoAVec` and exclusive references to its
    /// fields were exposed, then the `SoAVec` API would allow direct access to
    /// the `len` and `cap` fields that could be then used to mutate those
    /// without corresponding changes to `ptr`.
    ///
    /// In this case, the `Vec` should use a different `Mut` type that does not
    /// allow such mutations to occur.
    ///
    /// ```rust,ignore
    /// struct VecSoaRef<'a, T> {
    ///   ptr: &'a *mut T,
    ///   cap: &'a usize,
    ///   len: &'a usize,
    /// }
    ///
    /// impl<T> SoAble for AtomicVec<T> {
    ///   type Ref<'a> = VecSoARef<'a, T> where Self: 'a;
    /// }
    /// ```
    type Mut<'a>
    where
        Self: 'a;

    /// Representation of a group of the SoAble types as a group of slices
    /// borrowed from the Struct-of-Arrays.
    ///
    /// Generally this will be a tuple of slices matching the TupleRepr but
    /// in cases of types that split apart fields that have interconnected
    /// safety requirements that could be violated using shared references to
    /// individual fields, this type may be chosen to expose a safe interface
    /// over the group of field slices.
    type Slice<'a>: Copy
    where
        Self: 'a;

    /// Representation of a group of the SoAble types as a group of slices
    /// borrowed from the Struct-of-Arrays.
    ///
    /// Generally this will be a tuple of slices matching the TupleRepr but
    /// in cases of types that split apart fields that have interconnected
    /// safety requirements that could be violated using shared references to
    /// individual fields, this type may be chosen to expose a safe interface
    /// over the group of field slices.
    type SliceMut<'a>
    where
        Self: 'a;

    /// Take ownership of Self and convert to a tuple representation. The
    /// result will immediately be pushed into the SoAVec, which has already
    /// been checked to have space for the new item.
    ///
    /// ## Safety
    ///
    /// This method should never fail. The SoAVec guarantees that once the
    /// tuple has been received, it will always be successfully moved into the
    /// SoAVec and if dropping the value requires reconsituting a Self, then
    /// that will always be done on Drop.
    ///
    /// If the tuple representation gives unprivileged access to internal
    /// fields of the Self type that have safety invariants on them, then any
    /// of the Ref, Mut, Slice, and SliceMut types that enable violating those
    /// invariants should be manually defined.
    fn into_tuple(value: Self) -> Self::TupleRepr;
    /// Take ownership of Self's tuple representation and convert to Self. The
    /// result will immediately be given to the caller or dropped.
    fn from_tuple(value: Self::TupleRepr) -> Self;
    /// Convert the Self's pointer tuple representation into Ref.
    ///
    /// Note: this method should only perform the conversion and nothing else;
    /// the resulting code generation should effectively be a no-op.
    ///
    /// ### Safety
    ///
    /// The Ref type should not allow violating any safety invariants of Self.
    /// This is relevant for Ref types that expose internal mutation APIs.
    fn as_ref<'a>(
        _: PhantomData<&'a Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
    ) -> Self::Ref<'a>;
    /// Convert the Self's pointer tuple representation into Mut.
    ///
    /// Note: this method should only perform the conversion and nothing else;
    /// the resulting code generation should effectively be a no-op.
    ///
    /// ### Safety
    ///
    /// The Mut type should not allow violating any safety invariants of Self.
    fn as_mut<'a>(
        _: PhantomData<&'a mut Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
    ) -> Self::Mut<'a>;
    /// Convert the Self's pointer tuple representation with length into Slice.
    ///
    /// Note: this method should only perform the conversion and nothing else.
    ///
    /// ### Safety
    ///
    /// The Slice type should not allow violating any safety invariants of
    /// Self. This is relevant for Slice types that expose internal mutation
    /// APIs.
    fn as_slice<'a>(
        _: PhantomData<&'a Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
        len: u32,
    ) -> Self::Slice<'a>;
    /// Convert the Self's pointer tuple representation with length into
    /// SliceMut.
    ///
    /// Note: this method should only perform the conversion and nothing else.
    ///
    /// ### Safety
    ///
    /// The SliceMut type should not allow violating any safety invariants of
    /// Self.
    fn as_mut_slice<'a>(
        _: PhantomData<&'a mut Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
        len: u32,
    ) -> Self::SliceMut<'a>;
}

pub trait SoATuple {
    type Offsets: Copy;
    type Pointers: Copy;

    fn layout(capacity: u32) -> Result<Layout, LayoutError>;

    fn get_offsets(capacity: u32) -> Self::Offsets;

    unsafe fn grow(ptr: NonNull<u8>, new_capacity: u32, old_capacity: u32, len: u32);

    #[must_use]
    unsafe fn read(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self;

    unsafe fn push(ptr: NonNull<u8>, data: Self, len: u32, capacity: u32);

    unsafe fn copy(src: Self::Pointers, dst: Self::Pointers, count: u32);

    unsafe fn drop_in_place(ptrs: Self::Pointers, len: u32);

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

    unsafe fn read(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self {
        let ptr = Self::get_pointers(ptr, index, capacity);
        (ptr.0.read(), ptr.1.read())
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

    unsafe fn copy(src: Self::Pointers, dst: Self::Pointers, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        // SAFETY: old allocation; the layout has been checked.
        let (src_t_ptr, src_u_ptr) = src;
        let (dst_t_ptr, dst_u_ptr) = dst;

        // SAFETY: old data is located at src_*_ptr and its length is len
        unsafe {
            // Copy old data to new allocation area.
            core::ptr::copy(src_u_ptr.as_ptr(), dst_u_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_t_ptr.as_ptr(), dst_t_ptr.as_ptr(), len as usize);
        };
    }

    unsafe fn drop_in_place(ptrs: Self::Pointers, len: u32) {
        assert!(core::mem::needs_drop::<Self>());
        let (t_ptr, u_ptr) = ptrs;
        if core::mem::needs_drop::<T>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                t_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<U>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                u_ptr.as_ptr(),
                len as usize,
            ));
        }
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

    unsafe fn read(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self {
        let ptr = Self::get_pointers(ptr, index, capacity);
        (ptr.0.read(), ptr.1.read(), ptr.2.read())
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

    unsafe fn copy(src: Self::Pointers, dst: Self::Pointers, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        // SAFETY: old allocation; the layout has been checked.
        let (src_t_ptr, src_u_ptr, src_v_ptr) = src;
        let (dst_t_ptr, dst_u_ptr, dst_v_ptr) = dst;

        // SAFETY: old data is located at src_*_ptr and its length is len
        unsafe {
            // Copy old data to new allocation area.
            core::ptr::copy(src_v_ptr.as_ptr(), dst_v_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_u_ptr.as_ptr(), dst_u_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_t_ptr.as_ptr(), dst_t_ptr.as_ptr(), len as usize);
        };
    }

    unsafe fn drop_in_place(ptrs: Self::Pointers, len: u32) {
        assert!(core::mem::needs_drop::<Self>());
        let (t_ptr, u_ptr, v_ptr) = ptrs;
        if core::mem::needs_drop::<T>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                t_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<U>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                u_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<V>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                v_ptr.as_ptr(),
                len as usize,
            ));
        }
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

    unsafe fn read(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self {
        let ptr = Self::get_pointers(ptr, index, capacity);
        (ptr.0.read(), ptr.1.read(), ptr.2.read(), ptr.3.read())
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

    unsafe fn drop_in_place(ptrs: Self::Pointers, len: u32) {
        assert!(core::mem::needs_drop::<Self>());
        let (t_ptr, u_ptr, v_ptr, w_ptr) = ptrs;
        if core::mem::needs_drop::<T>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                t_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<U>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                u_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<V>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                v_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<W>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                w_ptr.as_ptr(),
                len as usize,
            ));
        }
    }

    unsafe fn copy(src: Self::Pointers, dst: Self::Pointers, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        // SAFETY: old allocation; the layout has been checked.
        let (src_t_ptr, src_u_ptr, src_v_ptr, src_w_ptr) = src;
        let (dst_t_ptr, dst_u_ptr, dst_v_ptr, dst_w_ptr) = dst;

        // SAFETY: old data is located at src_*_ptr and its length is len
        unsafe {
            // Copy old data to new allocation area.
            core::ptr::copy(src_w_ptr.as_ptr(), dst_w_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_v_ptr.as_ptr(), dst_v_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_u_ptr.as_ptr(), dst_u_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_t_ptr.as_ptr(), dst_t_ptr.as_ptr(), len as usize);
        };
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

    unsafe fn read(ptr: NonNull<u8>, index: u32, capacity: u32) -> Self {
        let ptr = Self::get_pointers(ptr, index, capacity);
        (
            ptr.0.read(),
            ptr.1.read(),
            ptr.2.read(),
            ptr.3.read(),
            ptr.4.read(),
        )
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

    unsafe fn copy(src: Self::Pointers, dst: Self::Pointers, len: u32) {
        if size_of::<Self>() == 0 || len == 0 {
            return;
        }
        // SAFETY: old allocation; the layout has been checked.
        let (src_t_ptr, src_u_ptr, src_v_ptr, src_w_ptr, src_x_ptr) = src;
        let (dst_t_ptr, dst_u_ptr, dst_v_ptr, dst_w_ptr, dst_x_ptr) = dst;

        // SAFETY: old data is located at src_*_ptr and its length is len
        unsafe {
            core::ptr::copy(src_t_ptr.as_ptr(), dst_t_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_x_ptr.as_ptr(), dst_x_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_w_ptr.as_ptr(), dst_w_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_v_ptr.as_ptr(), dst_v_ptr.as_ptr(), len as usize);
            // Copy old data to new allocation area.
            core::ptr::copy(src_u_ptr.as_ptr(), dst_u_ptr.as_ptr(), len as usize);
        };
    }

    unsafe fn drop_in_place(ptrs: Self::Pointers, len: u32) {
        assert!(core::mem::needs_drop::<Self>());
        let (t_ptr, u_ptr, v_ptr, w_ptr, x_ptr) = ptrs;
        if core::mem::needs_drop::<T>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                t_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<U>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                u_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<V>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                v_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<W>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                w_ptr.as_ptr(),
                len as usize,
            ));
        }
        if core::mem::needs_drop::<X>() {
            drop_in_place(core::ptr::slice_from_raw_parts_mut(
                x_ptr.as_ptr(),
                len as usize,
            ));
        }
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
    () => (
        compile_error!("soable macro requires explicit struct name, field names and types")
    );
    ($target:ident) => (
        compile_error!("soable macro requires explicit field names and types")
    );
    ($target:ident { $field:ident: $type:ty }) => (
        compile_error!("Single-field structs not supported; use a normal Vec")
    );
    ($target:ident { $($field:ident: $type:ty),+ }) => {
        unsafe impl SoAble for $target {
            type TupleRepr = ($($type),+);
            type Ref<'a> = ($(&'a $type),+);
            type Mut<'a> = ($(&'a mut $type),+);
            type Slice<'a> = ($(&'a [$type]),+);
            type SliceMut<'a> = ($(&'a mut [$type]),+);

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
            ) -> Self::Ref<'a> {
                let ($($field),+) = value;
                unsafe {
                    ($($field.as_ref()),+)
                }
            }

            fn as_mut<'a>(
                _: PhantomData<&'a mut Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
            ) -> Self::Mut<'a> {
                let ($(mut $field),+) = value;
                unsafe {
                    ($($field.as_mut()),+)
                }
            }

            fn as_slice<'a>(
                _: PhantomData<&'a Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
                len: u32,
            ) -> Self::Slice<'a> {
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
            ) -> Self::SliceMut<'a> {
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
    ($target:ident<$($lifetimes:lifetime),+> { $($field:ident: $type:ty),+ }) => {
        unsafe impl<$($lifetimes),+> SoAble for $target<'b> {
            type TupleRepr = ($($type),+);
            type Ref<'a> = ($(&'a $type),+) where Self: 'a;
            type Mut<'a> = ($(&'a mut $type),+) where Self: 'a;
            type Slice<'a> = ($(&'a [$type]),+) where Self: 'a;
            type SliceMut<'a> = ($(&'a mut [$type]),+) where Self: 'a;

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
            ) -> Self::Ref<'a> {
                let ($($field),+) = value;
                unsafe {
                    ($($field.as_ref()),+)
                }
            }

            fn as_mut<'a>(
                _: PhantomData<&'a mut Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
            ) -> Self::Mut<'a> {
                let ($(mut $field),+) = value;
                unsafe {
                    ($($field.as_mut()),+)
                }
            }

            fn as_slice<'a>(
                _: PhantomData<&'a Self>,
                value: <Self::TupleRepr as SoATuple>::Pointers,
                len: u32,
            ) -> Self::Slice<'a> {
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
            ) -> Self::SliceMut<'a> {
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

unsafe impl<T, U> SoAble for (T, U) {
    type TupleRepr = Self;

    type Ref<'a>
        = (&'a T, &'a U)
    where
        Self: 'a;

    type Mut<'a>
        = (&'a mut T, &'a mut U)
    where
        Self: 'a;

    type Slice<'a>
        = (&'a [T], &'a [U])
    where
        Self: 'a;

    type SliceMut<'a>
        = (&'a mut [T], &'a mut [U])
    where
        Self: 'a;

    fn into_tuple(value: Self) -> Self::TupleRepr {
        value
    }

    fn from_tuple(value: Self::TupleRepr) -> Self {
        value
    }

    fn as_ref<'a>(
        _: PhantomData<&'a Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
    ) -> Self::Ref<'a> {
        let (a, b) = value;
        unsafe { (a.as_ref(), b.as_ref()) }
    }

    fn as_mut<'a>(
        _: PhantomData<&'a mut Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
    ) -> Self::Mut<'a> {
        let (mut a, mut b) = value;
        unsafe { (a.as_mut(), b.as_mut()) }
    }

    fn as_slice<'a>(
        _: PhantomData<&'a Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
        len: u32,
    ) -> Self::Slice<'a> {
        let len = len as usize;
        let (a, b) = value;
        unsafe {
            (
                core::slice::from_raw_parts(a.as_ptr(), len),
                core::slice::from_raw_parts(b.as_ptr(), len),
            )
        }
    }

    fn as_mut_slice<'a>(
        _: PhantomData<&'a mut Self>,
        value: <Self::TupleRepr as SoATuple>::Pointers,
        len: u32,
    ) -> Self::SliceMut<'a> {
        let len = len as usize;
        let (a, b) = value;
        unsafe {
            (
                core::slice::from_raw_parts_mut(a.as_ptr(), len),
                core::slice::from_raw_parts_mut(b.as_ptr(), len),
            )
        }
    }
}
