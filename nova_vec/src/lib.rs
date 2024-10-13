mod raw_vec;
mod raw_vec_inner;

use raw_vec::NovaRawVec2;

pub struct NovaVec2<T: Sized, U: Sized> {
    buf: NovaRawVec2<T, U>,
    len: u32,
}

impl<T: Sized, U: Sized> NovaVec2<T, U> {
    pub const fn new() -> Self {
        NovaVec2 {
            buf: NovaRawVec2::NEW,
            len: 0,
        }
    }

    pub fn with_capacity(cap: u32) -> Self {
        if cap == 0 {
            return Self::new();
        }
        NovaVec2 {
            buf: NovaRawVec2::with_capacity(cap),
            len: 0,
        }
    }

    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn capacity(&self) -> u32 {
        self.buf.capacity()
    }

    pub fn reserve(&mut self, additional: u32) {
        self.buf.reserve(self.len, additional);
    }

    pub fn push(&mut self, value: (T, U)) {}
}

#[cfg(test)]
mod tests {
    use crate::NovaVec2;

    #[test]
    fn it_works() {
        let data = NovaVec2::<u32, u16>::with_capacity(16);
    }
}
