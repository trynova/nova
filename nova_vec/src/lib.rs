mod raw_vec;
mod raw_vec_inner;
mod soable;

use std::ptr::NonNull;

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
        unsafe { T::TupleRepr::push(self.as_mut_ptr(), T::to_tuple(value), len, self.capacity()) };
        // SAFETY: length cannot overflow due to reserve succeeding.
        self.buf.set_len(unsafe { self.len().unchecked_add(1) });
        Ok(())
    }

    // pub fn get<'a>(&'a self, index: u32) -> Option<&'a T::Ref<'a>> {
    //     if self.len() <= index {
    //         // Over-indexing.
    //         return None;
    //     }
    //     unsafe { T::TupleRepr::get_pointers(self.as_ptr(), index, self.capacity()) };
    //     Some(T::from_tuple())
    // }

    fn as_ptr(&self) -> NonNull<u8> {
        self.buf.as_ptr()
    }

    fn as_mut_ptr(&mut self) -> NonNull<u8> {
        self.buf.as_mut_ptr()
    }
}

#[cfg(test)]
mod tests {
    use crate::{soable::SoAble, SoAVec};

    #[test]
    fn basic_usage() {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        struct Foo {
            a: u64,
            b: u32,
        }

        impl SoAble for Foo {
            type TupleRepr = (u64, u32);

            fn to_tuple(value: Self) -> Self::TupleRepr {
                let Self { a, b } = value;
                (a, b)
            }

            fn from_tuple(value: Self::TupleRepr) -> Self {
                let (a, b) = value;
                Self { a, b }
            }
        }

        let mut foo = SoAVec::<Foo>::with_capacity(16).unwrap();
        foo.reserve(32).unwrap();
        foo.push(Foo { a: 0, b: 2 }).unwrap();
        // let first = foo.get_cloned(0).unwrap();
        // debug_assert_eq!(first.a, 0);
        // debug_assert_eq!(first.b, 2);
        // let a_0: &u64 = foo.get_a(0);
        // let a_0: &u32 = foo.get_b(0);
        // let a_n: &[u64] = foo.get_all_a();
    }

    // fn more_basic_usage() {
    //     #[repr(C)]
    //     #[derive(Debug, Clone, Copy)]
    //     struct Bar {
    //         a: u64,
    //         b: u32,
    //         c: u8,
    //     }

    //     impl SoAble for Bar {
    //         type TupleRepr = (u64, u32, u8);
    //         type Ref<'a> = (&'a u64, &'a u32);
    //         type Mut<'a> = (&'a mut u64, &'a mut u32);

    //         fn to_tuple(value: Self) -> Self::TupleRepr {
    //             let Self { a, b, c } = value;
    //             (a, b, c)
    //         }

    //         fn from_tuple(value: Self::TupleRepr) -> Self {
    //             let (a, b, c) = value;
    //             Self { a, b, c }
    //         }
    //     }

    //     /// Conceptually; this is what we're doing here.
    //     const _ARRAY: [Foo; 16] = [Foo { a: 0, b: 1 }; 16];
    //     const _SOA_ARRAY: ([u64; 16], [u32; 16]) = ([0; 16], [1; 16]);

    //     let mut bar = SoAVec::<Bar>::with_capacity(16).unwrap();
    //     bar.reserve(32).unwrap();
    //     bar.push(Bar { a: 0, b: 2, c: 255 }).unwrap();
    //     let first = bar.get_cloned(0).unwrap();
    //     debug_assert_eq!(first.a, 0);
    //     debug_assert_eq!(first.b, 2);
    //     debug_assert_eq!(first.c, 255);
    // }

    // #[test]
    // fn basic_usage_with_bad_alignment() {
    //     #[repr(C)]
    //     #[derive(Debug, Clone, Copy)]
    //     struct Foo {
    //         b: u32,
    //         a: u64,
    //     }

    //     impl SoAble for Foo {
    //         type TupleRepr = (u32, u64);

    //         fn to_tuple(value: Self) -> Self::TupleRepr {
    //             let Self { a, b } = value;
    //             (b, a)
    //         }

    //         fn from_tuple(value: Self::TupleRepr) -> Self {
    //             let (b, a) = value;
    //             Self { b, a }
    //         }
    //     }

    //     #[repr(C)]
    //     #[derive(Debug, Clone, Copy)]
    //     struct Bar {
    //         c: u8,
    //         b: u32,
    //         a: u64,
    //     }

    //     impl SoAble for Bar {
    //         type TupleRepr = (u8, u32, u64);

    //         fn to_tuple(value: Self) -> Self::TupleRepr {
    //             let Self { c, b, a } = value;
    //             (c, b, a)
    //         }

    //         fn from_tuple(value: Self::TupleRepr) -> Self {
    //             let (c, b, a) = value;
    //             Self { c, b, a }
    //         }
    //     }

    //     /// Conceptually; this is what we're doing here.
    //     const _ARRAY: [Foo; 16] = [Foo { a: 0, b: 1 }; 16];
    //     const _SOA_ARRAY: ([u64; 16], [u32; 16]) = ([0; 16], [1; 16]);

    //     let mut foo = SoAVec::<Foo>::with_capacity(5).unwrap();
    //     foo.reserve(9).unwrap();
    //     foo.push(Foo { a: 0, b: 2 }).unwrap();
    //     let first = foo.get_cloned(0).unwrap();
    //     debug_assert_eq!(first.a, 0);
    //     debug_assert_eq!(first.b, 2);
    //     // let a_0: &u64 = foo.get_a(0);
    //     // let a_0: &u32 = foo.get_b(0);
    //     // let a_n: &[u64] = foo.get_all_a();

    //     let mut bar = SoAVec::<Bar>::with_capacity(7).unwrap();
    //     bar.reserve(11).unwrap();
    //     bar.push(Bar { a: 0, b: 2, c: 255 }).unwrap();
    //     let first = bar.get_cloned(0).unwrap();
    //     debug_assert_eq!(first.a, 0);
    //     debug_assert_eq!(first.b, 2);
    //     debug_assert_eq!(first.c, 255);
    // }

    // #[test]
    // fn basic_usage_with_zst() {
    //     #[repr(C)]
    //     #[derive(Debug, Clone, Copy)]
    //     struct Foo {
    //         b: u32,
    //         a: (),
    //     }

    //     impl SoAble for Foo {
    //         type TupleRepr = (u32, ());

    //         fn to_tuple(value: Self) -> Self::TupleRepr {
    //             let Self { a, b } = value;
    //             (b, a)
    //         }

    //         fn from_tuple(value: Self::TupleRepr) -> Self {
    //             let (b, a) = value;
    //             Self { b, a }
    //         }
    //     }

    //     #[repr(C)]
    //     #[derive(Debug, Clone, Copy)]
    //     struct Bar {
    //         c: u8,
    //         b: (),
    //         a: u64,
    //     }

    //     impl SoAble for Bar {
    //         type TupleRepr = (u8, (), u64);

    //         fn to_tuple(value: Self) -> Self::TupleRepr {
    //             let Self { c, b, a } = value;
    //             (c, b, a)
    //         }

    //         fn from_tuple(value: Self::TupleRepr) -> Self {
    //             let (c, b, a) = value;
    //             Self { c, b, a }
    //         }
    //     }

    //     #[repr(C)]
    //     #[derive(Debug, Clone, Copy)]
    //     struct Baz {
    //         c: (),
    //         b: (),
    //         a: (),
    //     }

    //     impl SoAble for Baz {
    //         type TupleRepr = ((), (), ());

    //         fn to_tuple(value: Self) -> Self::TupleRepr {
    //             let Self { c, b, a } = value;
    //             (c, b, a)
    //         }

    //         fn from_tuple(value: Self::TupleRepr) -> Self {
    //             let (c, b, a) = value;
    //             Self { c, b, a }
    //         }
    //     }

    //     let mut foo = SoAVec::<Foo>::with_capacity(5).unwrap();
    //     foo.reserve(9).unwrap();
    //     foo.push(Foo { a: (), b: 2 }).unwrap();
    //     let first = foo.get_cloned(0).unwrap();
    //     debug_assert_eq!(first.a, ());
    //     debug_assert_eq!(first.b, 2);

    //     let mut bar = SoAVec::<Bar>::with_capacity(7).unwrap();
    //     bar.reserve(11).unwrap();
    //     bar.push(Bar {
    //         a: 0,
    //         b: (),
    //         c: 255,
    //     })
    //     .unwrap();
    //     let first = bar.get_cloned(0).unwrap();
    //     debug_assert_eq!(first.a, 0);
    //     debug_assert_eq!(first.b, ());
    //     debug_assert_eq!(first.c, 255);

    //     let mut baz = SoAVec::<Baz>::with_capacity(7).unwrap();
    //     baz.reserve(11).unwrap();
    //     baz.push(Baz {
    //         a: (),
    //         b: (),
    //         c: (),
    //     })
    //     .unwrap();
    //     let first = baz.get_cloned(0).unwrap();
    //     debug_assert_eq!(first.a, ());
    //     debug_assert_eq!(first.b, ());
    //     debug_assert_eq!(first.c, ());
    // }
}
