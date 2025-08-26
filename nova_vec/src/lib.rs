mod raw_vec;
mod raw_vec_inner;
mod soable;

use std::{marker::PhantomData, ptr::NonNull};

use raw_vec::RawSoAVec;
use raw_vec_inner::AllocError;
use soable::{SoATuple, SoAble};

pub struct SoAVec<T: SoAble> {
    buf: RawSoAVec<T>,
}

impl<T: SoAble> SoAVec<T> {
    pub fn with_capacity(cap: u32) -> Result<Self, AllocError> {
        Ok(SoAVec {
            buf: RawSoAVec::with_capacity(cap)?,
        })
    }

    pub fn len(&self) -> u32 {
        self.buf.len()
    }

    pub fn capacity(&self) -> u32 {
        self.buf.capacity()
    }

    pub fn reserve(&mut self, additional: u32) -> Result<(), AllocError> {
        self.buf.reserve(additional)
    }

    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        let len = self.len();
        if len == self.capacity() {
            self.buf.reserve(1)?;
        }

        // SAFETY: sure.
        unsafe {
            T::TupleRepr::push(
                self.as_mut_ptr(),
                T::into_tuple(value),
                len,
                self.capacity(),
            )
        };
        // SAFETY: length cannot overflow due to reserve succeeding.
        self.buf.set_len(unsafe { self.len().unchecked_add(1) });
        Ok(())
    }

    fn as_ptr(&self) -> NonNull<u8> {
        self.buf.as_ptr()
    }

    fn as_mut_ptr(&mut self) -> NonNull<u8> {
        self.buf.as_mut_ptr()
    }

    pub fn get(&self, index: u32) -> Option<T::RefTuple<'_>> {
        if self.len() <= index {
            // Over-indexing.
            return None;
        }
        let ptrs = unsafe { T::TupleRepr::get_pointers(self.as_ptr(), index, self.capacity()) };
        Some(T::as_ref(PhantomData, ptrs))
    }

    pub fn get_mut(&mut self, index: u32) -> Option<T::MutTuple<'_>> {
        if self.len() <= index {
            // Over-indexing.
            return None;
        }
        let ptrs = unsafe { T::TupleRepr::get_pointers(self.as_mut_ptr(), index, self.capacity()) };
        Some(T::as_mut(PhantomData, ptrs))
    }

    pub fn as_slice(&self) -> T::SliceTuple<'_> {
        let ptrs = unsafe { T::TupleRepr::get_pointers(self.as_ptr(), 0, self.capacity()) };
        let len = self.len();
        T::as_slice(PhantomData, ptrs, len)
    }

    pub fn as_mut_slice(&mut self) -> T::SliceMutTuple<'_> {
        let ptrs = unsafe { T::TupleRepr::get_pointers(self.as_ptr(), 0, self.capacity()) };
        let len = self.len();
        T::as_mut_slice(PhantomData, ptrs, len)
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

        /// Conceptually; this is what we're doing here.
        const _ARRAY: [Foo; 16] = [Foo { a: 0, b: 1 }; 16];
        const _SOA_ARRAY: ([u64; 16], [u32; 16]) = ([0; 16], [1; 16]);

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
}
