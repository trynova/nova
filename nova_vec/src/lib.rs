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

    pub fn get_cloned(&self, index: u32) -> Option<T> {
        if self.len() <= index {
            // Over-indexing.
            return None;
        }
        Some(T::from_tuple(unsafe {
            T::TupleRepr::get_cloned(self.as_ptr(), index, self.capacity())
        }))
    }

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

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct Foo {
        a: u64,
        b: u32,
    }

    impl SoAble for Foo {
        type TupleRepr = (u64, u32);

        fn to_tuple(value: Self) -> Self::TupleRepr {
            (value.a, value.b)
        }

        fn from_tuple(value: Self::TupleRepr) -> Self {
            Self {
                a: value.0,
                b: value.1,
            }
        }
    }

    const _ARRAY: [Foo; 16] = [Foo { a: 0, b: 1 }; 16];
    const _SOA_ARRAY: ([u64; 16], [u32; 16]) = ([0; 16], [1; 16]);

    #[test]
    fn basic_create_reserve_push_get_cloned() {
        let mut foo = SoAVec::<Foo>::with_capacity(16).unwrap();
        foo.reserve(32).unwrap();
        foo.push(Foo { a: 0, b: 2 }).unwrap();
        let first = foo.get_cloned(0).unwrap();
        debug_assert_eq!(first.a, 0);
        debug_assert_eq!(first.b, 2);
        // let a_0: &u64 = foo.get_a(0);
        // let a_0: &u32 = foo.get_b(0);
        // let a_n: &[u64] = foo.get_all_a();
    }
}
