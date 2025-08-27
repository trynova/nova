mod macros;
mod raw_vec;
mod raw_vec_inner;
mod soable;

use std::marker::PhantomData;

use raw_vec::RawSoAVec;
use raw_vec_inner::AllocError;
use soable::{SoATuple, SoAble};

pub struct SoAVec<T: SoAble> {
    buf: RawSoAVec<T>,
}

impl<T: SoAble> SoAVec<T> {
    pub fn new() -> Self {
        SoAVec {
            // SAFETY: 0-capacity vector cannot create an invalid layout.
            buf: unsafe { RawSoAVec::with_capacity(0).unwrap_unchecked() },
        }
    }

    /// Constructs a new, empty `SoAVec<T>` with at least the specified
    /// capacity. Returns an error if an allocator error occurred.
    ///
    /// The Struct-of-Arrays vector will be able to hold at least `capacity`
    /// elements without reallocating. This method is allowed to allocate for
    /// more elements than `capacity`. If `capacity` is zero, the vector will
    /// not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// minimum *capacity* specified, the vector will have a zero *length*. For
    /// an explanation of the difference between length and capacity, see
    /// *[Capacity and reallocation]*.
    ///
    /// If it is important to know the exact allocated capacity of a `SoAVec`,
    /// always use the [`capacity`] method after construction.
    ///
    /// For `SoAVec<T>` where `T` is a zero-sized type, there will be no
    /// allocation and the capacity will always be `usize::MAX`.
    ///
    /// [Capacity and reallocation]: #capacity-and-reallocation
    /// [`capacity`]: Vec::capacity
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::SoAVec;
    ///
    /// let mut vec = SoAVec::<(u32, u32)>::with_capacity(10).unwrap();
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push((i, i));
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push((11, 11));
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    ///
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = SoAVec::<((), ())>::with_capacity(10).unwrap();
    /// assert_eq!(vec_units.capacity(), u32::MAX);
    /// ```
    pub fn with_capacity(cap: u32) -> Result<Self, AllocError> {
        Ok(SoAVec {
            buf: RawSoAVec::with_capacity(cap)?,
        })
    }

    /// Returns the number of elements in the vector, also referred to
    /// as its 'length'.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::soavec;
    ///
    /// let mut a = soavec![(0, 0); 3].unwrap();
    /// assert_eq!(a.len(), 3);
    /// ```
    pub fn len(&self) -> u32 {
        self.buf.len()
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::SoAVec;
    ///
    /// let mut v = SoAVec::new();
    /// assert!(v.is_empty());
    ///
    /// v.push((1, 1));
    /// assert!(!v.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::SoAVec;
    ///
    /// let mut vec: SoAVec<(u32, i32)> = SoAVec::with_capacity(10).unwrap();
    /// vec.push((42, 3));
    /// assert!(vec.capacity() >= 10);
    /// ```
    ///
    /// A vector with zero-sized elements will always have a capacity of usize::MAX:
    ///
    /// ```
    /// use nova_vec::SoAVec;
    ///
    /// fn main() {
    ///     assert_eq!(std::mem::size_of::<((), ())>(), 0);
    ///     let v = SoAVec::<((), ())>::new();
    ///     assert_eq!(v.capacity(), u32::MAX);
    /// }
    /// ```
    pub fn capacity(&self) -> u32 {
        self.buf.capacity()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `Vec<T>`. The collection may reserve more space to
    /// speculatively avoid frequent reallocations. After calling `reserve`,
    /// capacity will be greater than or equal to `self.len() + additional`.
    /// Does nothing if capacity is already sufficient.
    ///
    /// # Errors
    ///
    /// Returns an error if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::soavec;
    ///
    /// let mut vec = soavec![(1u32, 1u32)].unwrap();
    /// vec.reserve(10).unwrap();
    /// assert!(vec.capacity() >= 11);
    /// ```
    pub fn reserve(&mut self, additional: u32) -> Result<(), AllocError> {
        self.buf.reserve(additional)
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::soavec;
    ///
    /// let mut vec = soavec![(1u32, 1u32), (2u32, 2u32)].unwrap();
    /// vec.push((3, 3)).unwrap();
    /// assert_eq!(vec.get(0), Some((&1, &1)));
    /// assert_eq!(vec.get(1), Some((&2, &2)));
    /// assert_eq!(vec.get(2), Some((&3, &3)));
    /// ```
    ///
    /// # Time complexity
    ///
    /// Takes amortized *O*(1) time. If the vector's length would exceed its
    /// capacity after the push, *O*(*capacity*) time is taken to copy the
    /// vector's elements to a larger allocation. This expensive operation is
    /// offset by the *capacity* *O*(1) insertions it allows.
    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        let len = self.len();
        if len == self.capacity() {
            self.buf.reserve(1)?;
        }

        // SAFETY: sure.
        unsafe {
            T::TupleRepr::push(
                self.buf.as_mut_ptr(),
                T::into_tuple(value),
                len,
                self.capacity(),
            )
        };
        // SAFETY: length cannot overflow due to reserve succeeding.
        self.buf.set_len(unsafe { self.len().unchecked_add(1) });
        Ok(())
    }

    /// Returns a reference to each field in T or `None` if out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::soavec;
    ///
    /// let v = soavec![(10, 10), (40, 40), (30, 30)].unwrap();
    /// assert_eq!(Some((&40, &40)), v.get(1));
    /// assert_eq!(None, v.get(3));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, index: u32) -> Option<T::Ref<'_>> {
        if self.len() <= index {
            // Over-indexing.
            return None;
        }
        let ptrs = unsafe { T::TupleRepr::get_pointers(self.buf.as_ptr(), index, self.capacity()) };
        Some(T::as_ref(PhantomData, ptrs))
    }

    /// Returns a mutable reference to each field in `T` or `None` if the index
    /// is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::soavec;
    ///
    /// let x = &mut soavec![(0, 0), (1, 1), (2, 2)].unwrap();
    ///
    /// if let Some((first, second)) = x.get_mut(1) {
    ///     *first = 42;
    ///     *second = 3;
    /// }
    /// assert_eq!(x.as_slice(), ([0, 42, 2].as_slice(), [0, 3, 2].as_slice()));
    /// ```
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, index: u32) -> Option<T::Mut<'_>> {
        if self.len() <= index {
            // Over-indexing.
            return None;
        }
        let ptrs =
            unsafe { T::TupleRepr::get_pointers(self.buf.as_mut_ptr(), index, self.capacity()) };
        Some(T::as_mut(PhantomData, ptrs))
    }

    /// Extracts a tuple of slices containing the entire vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::soavec;
    /// use std::io::{self, Write};
    ///
    /// let buffer = soavec![(1, 1), (2, 2), (3, 3), (5, 5), (8, 8)].unwrap();
    /// let (first, second) = buffer.as_slice();
    /// io::sink().write(first).unwrap();
    /// io::sink().write(second).unwrap();
    /// ```
    pub fn as_slice(&self) -> T::Slice<'_> {
        let ptrs = unsafe { T::TupleRepr::get_pointers(self.buf.as_ptr(), 0, self.capacity()) };
        let len = self.len();
        T::as_slice(PhantomData, ptrs, len)
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use nova_vec::soavec;
    /// use std::io::{self, Read};
    /// let mut buffer = soavec![(0, 0); 3].unwrap();
    /// let (first, second) = buffer.as_mut_slice();
    ///
    /// io::repeat(0b101).read_exact(first).unwrap();
    /// io::repeat(0b010).read_exact(second).unwrap();
    /// ```
    pub fn as_mut_slice(&mut self) -> T::SliceMut<'_> {
        let ptrs = unsafe { T::TupleRepr::get_pointers(self.buf.as_ptr(), 0, self.capacity()) };
        let len = self.len();
        T::as_mut_slice(PhantomData, ptrs, len)
    }
}

impl<T: SoAble> Drop for SoAVec<T> {
    fn drop(&mut self) {
        if !core::mem::needs_drop::<T>() {
            return;
        }
        let ptr = self.buf.as_ptr();
        let cap = self.buf.capacity();
        let len = self.len();
        for i in 0..len {
            // SAFETY: reads each value out without altering the backing
            // memory; using the backing memory may violate memory safety
            // after this but we are about to deallocate it afterwards.
            let _ = T::from_tuple(unsafe { T::TupleRepr::read(ptr, i, cap) });
        }
        // RawVec handles deallocation
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use crate::{soable, SoATuple, SoAVec, SoAble};

    #[test]
    fn basic_usage() {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Foo {
            a: u64,
            b: u32,
        }
        soable!(Foo { a: u64, b: u32 });

        /// Conceptually; this is what we're doing here.
        const _ARRAY: [Foo; 16] = [Foo { a: 0, b: 1 }; 16];
        const _SOA_ARRAY: ([u64; 16], [u32; 16]) = ([0; 16], [1; 16]);

        let mut foo = SoAVec::<Foo>::with_capacity(16).unwrap();
        foo.push(Foo { a: 0, b: 2 }).unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &0);
        debug_assert_eq!(first.1, &2);

        let first = foo.get_mut(0).unwrap();
        *first.0 = 52;
        *first.1 = 66;
        debug_assert_eq!(first.0, &52);
        debug_assert_eq!(first.1, &66);

        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &52);
        debug_assert_eq!(first.1, &66);

        foo.reserve(32).unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &52);
        debug_assert_eq!(first.1, &66);

        foo.push(Foo { a: 4, b: 8 }).unwrap();
        let (a_slice, b_slice) = foo.as_slice();
        debug_assert_eq!(a_slice.len(), b_slice.len());
        debug_assert_eq!(a_slice.len(), 2);
        debug_assert_eq!(a_slice, &[52, 4]);
        debug_assert_eq!(b_slice, &[66, 8]);
    }

    #[test]
    fn basic_usage_with_lifetime() {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Foo<'a> {
            a: &'a u64,
            b: &'a u32,
        }
        soable!(Foo<'b> { a: &'b u64, b: &'b u32 });

        let mut foo = SoAVec::<Foo>::with_capacity(16).unwrap();
        foo.push(Foo { a: &0, b: &2 }).unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &&0);
        debug_assert_eq!(first.1, &&2);

        let first = foo.get_mut(0).unwrap();
        *first.0 = &52;
        *first.1 = &66;
        debug_assert_eq!(first.0, &&52);
        debug_assert_eq!(first.1, &&66);

        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &&52);
        debug_assert_eq!(first.1, &&66);

        foo.reserve(32).unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &&52);
        debug_assert_eq!(first.1, &&66);

        foo.push(Foo { a: &4, b: &8 }).unwrap();
        let (a_slice, b_slice) = foo.as_slice();
        debug_assert_eq!(a_slice.len(), b_slice.len());
        debug_assert_eq!(a_slice.len(), 2);
        debug_assert_eq!(a_slice, &[&52, &4]);
        debug_assert_eq!(b_slice, &[&66, &8]);
    }

    #[test]
    fn more_basic_usage() {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Bar {
            a: u64,
            b: u32,
            c: u8,
        }
        soable!(Bar {
            a: u64,
            b: u32,
            c: u8
        });

        let mut bar = SoAVec::<Bar>::with_capacity(16).unwrap();
        bar.reserve(32).unwrap();
        bar.push(Bar { a: 0, b: 2, c: 255 }).unwrap();
        let first = bar.get(0).unwrap();
        debug_assert_eq!(first.0, &0);
        debug_assert_eq!(first.1, &2);
        debug_assert_eq!(first.2, &255);
    }

    #[test]
    fn basic_usage_with_bad_alignment() {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Foo {
            b: u32,
            a: u64,
        }
        soable!(Foo { b: u32, a: u64 });

        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Bar {
            c: u8,
            b: u32,
            a: u64,
        }
        soable!(Bar {
            c: u8,
            b: u32,
            a: u64
        });

        let mut foo = SoAVec::<Foo>::with_capacity(5).unwrap();
        foo.reserve(9).unwrap();
        foo.push(Foo { b: 2, a: 0 }).unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &2);
        debug_assert_eq!(first.1, &0);
        // let a_0: &u64 = foo.get_a(0);
        // let a_0: &u32 = foo.get_b(0);
        // let a_n: &[u64] = foo.get_all_a();

        let mut bar = SoAVec::<Bar>::with_capacity(7).unwrap();
        bar.reserve(11).unwrap();
        bar.push(Bar { c: 255, b: 2, a: 0 }).unwrap();
        let first = bar.get(0).unwrap();
        debug_assert_eq!(first.0, &255);
        debug_assert_eq!(first.1, &2);
        debug_assert_eq!(first.2, &0);
    }

    #[test]
    fn basic_usage_with_zst() {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Foo {
            b: (),
            a: u32,
        }
        soable!(Foo { b: (), a: u32 });

        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Bar {
            c: u8,
            b: (),
            a: u64,
        }
        soable!(Bar {
            c: u8,
            b: (),
            a: u64
        });

        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Baz {
            c: (),
            b: (),
            a: (),
        }
        soable!(Baz {
            c: (),
            b: (),
            a: ()
        });

        let mut foo = SoAVec::<Foo>::with_capacity(5).unwrap();
        foo.reserve(9).unwrap();
        foo.push(Foo { a: 2, b: () }).unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &());
        debug_assert_eq!(first.1, &2);

        let mut bar = SoAVec::<Bar>::with_capacity(7).unwrap();
        bar.reserve(11).unwrap();
        bar.push(Bar {
            c: 255,
            b: (),
            a: 0,
        })
        .unwrap();
        let first = bar.get(0).unwrap();
        debug_assert_eq!(first.0, &255);
        debug_assert_eq!(first.1, &());
        debug_assert_eq!(first.2, &0);

        let mut baz = SoAVec::<Baz>::with_capacity(7).unwrap();
        baz.reserve(11).unwrap();
        baz.push(Baz {
            a: (),
            b: (),
            c: (),
        })
        .unwrap();
        let first = baz.get(0).unwrap();
        debug_assert_eq!(first.0, &());
        debug_assert_eq!(first.1, &());
        debug_assert_eq!(first.2, &());
    }

    #[test]
    fn droppable_types() {
        #[repr(C)]
        #[derive(Debug, Clone)]
        struct Foo {
            a: Vec<u64>,
            b: Box<u32>,
        }
        soable!(Foo { a: Vec<u64>, b: Box<u32> });

        let mut foo = SoAVec::<Foo>::with_capacity(16).unwrap();
        foo.push(Foo {
            a: vec![0],
            b: Box::new(2),
        })
        .unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &[0]);
        debug_assert_eq!(**first.1, 2);

        let first = foo.get_mut(0).unwrap();
        first.0.push(52);
        *first.1 = Box::new(66u32);
        debug_assert_eq!(first.0, &[0, 52]);
        debug_assert_eq!(**first.1, 66u32);

        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &[0, 52]);
        debug_assert_eq!(**first.1, 66u32);

        foo.reserve(32).unwrap();
        let first = foo.get(0).unwrap();
        debug_assert_eq!(first.0, &[0, 52]);
        debug_assert_eq!(**first.1, 66u32);

        foo.push(Foo {
            a: vec![4],
            b: Box::new(8),
        })
        .unwrap();
        let (a_slice, b_slice) = foo.as_slice();
        debug_assert_eq!(a_slice.len(), b_slice.len());
        debug_assert_eq!(a_slice.len(), 2);
        debug_assert_eq!(a_slice, &[vec![0, 52], vec![4]]);
        debug_assert_eq!(b_slice, &[Box::new(66), Box::new(8)]);
    }
}
