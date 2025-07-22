use std::{cell::Cell, fmt, marker::PhantomData, ops};

use super::*;
// type Ptr<'a, T: ?Sized> = BaseIndex<'a, T>;

pub trait SubspaceResident<'a, HeapRepr: ?Sized> {
    type Space: Subspace<'a, HeapRepr, Self>;
    fn subspace_for(heap: &Heap) -> &Self::Space;
    fn subspace_for_mut(heap: &mut Heap) -> &mut Self::Space;
}

pub trait IsoSubspaceResident {
    type Data<'a>: Bindable<Of<'a> = Self::Data<'a>>;
}

macro_rules! declare_subspace_resident {
    (iso; struct $Nominal:ident, $Data:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $Nominal<'a>(BaseIndex<'a, $Data<'static>>);

        impl<'a> $Nominal<'a> {
            /// # Do not use this
            /// This is only for Value discriminant creation.
            pub(crate) const fn _def() -> Self {
                Self(BaseIndex::from_u32_index(0))
            }
            pub(crate) fn get_index(self) -> usize {
                self.0.into_index()
            }
        }

        impl<'a> From<BaseIndex<'a, $Data<'static>>> for $Nominal<'a> {
            fn from(value: BaseIndex<'a, $Data<'static>>) -> Self {
                $Nominal(value)
            }
        }

        // SAFETY: Property implemented as a lifetime transmute.
        unsafe impl crate::engine::context::Bindable for $Nominal<'_> {
            type Of<'a> = $Nominal<'a>;

            #[inline(always)]
            fn unbind(self) -> Self::Of<'static> {
                unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
            }

            #[inline(always)]
            fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
                unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
            }
        }

        impl crate::heap::IsoSubspaceResident for $Nominal<'_> {
            type Data<'a> = $Data<'a>;
        }
    };
}
pub(crate) use declare_subspace_resident;

pub trait Subspace<'a, T: ?Sized, Ptr: ?Sized> {
    fn alloc(&'a mut self, data: T) -> Ptr;
}

pub struct IsoSubspace<T, D> {
    name: &'static str,
    alloc_count: usize,
    data: Vec<Option<D>>,
    _marker: PhantomData<T>,
}

impl<T, D> IsoSubspace<T, D> {
    fn new(name: &'static str) -> Self {
        Self::with_capacity(name, 0)
    }

    pub fn with_capacity(name: &'static str, capacity: usize) -> Self {
        Self {
            name,
            alloc_count: 0,
            data: Vec::with_capacity(capacity),
            _marker: PhantomData,
        }
    }
}

impl<T, D> IsoSubspace<T, D>
where
    T: Into<usize>,
{
    pub fn get(&self, key: T) -> Option<&D> {
        self.data.get(key.into()).map(Option::as_ref).flatten()
    }
    pub fn get_mut(&mut self, key: T) -> Option<&mut D> {
        self.data.get_mut(key.into()).map(Option::as_mut).flatten()
    }
}

impl<T, D> ops::Index<T> for IsoSubspace<T, D>
where
    T: Into<usize>,
{
    type Output = D;
    fn index(&self, index: T) -> &Self::Output {
        self.data
            .get(index.into())
            .expect("subspace index out of bounds")
            .as_ref()
            .expect("subspace slot is empty")
    }
}
impl<T, D> ops::IndexMut<T> for IsoSubspace<T, D>
where
    T: Into<usize>,
{
    fn index_mut(&mut self, index: T) -> &mut Self::Output {
        self.data
            .get_mut(index.into())
            .expect("subspace index out of bounds")
            .as_mut()
            .expect("subspace slot is empty")
    }
}

// impl<T, D> Subspace<D> for IsoSubspace<T, D>
// {
//     fn alloc(&mut self, data: D) -> Ptr<'_, D> {
//         self.data.push(Some(data));
//         self.alloc_count += core::mem::size_of::<D>();
//         return Ptr::from_usize(self.data.len());
//     }
// }

impl<'a, R> Subspace<'a, R::Data<'a>, R> for IsoSubspace<R, R::Data<'static>>
where
    R: IsoSubspaceResident + From<BaseIndex<'a, R::Data<'static>>>,
{
    fn alloc(&mut self, data: R::Data<'a>) -> R {
        // SAFETY: this is not safe. fixme.
        let d: R::Data<'static> = unsafe { std::mem::transmute(data) };
        self.data.push(Some(d));
        self.alloc_count += core::mem::size_of::<R::Data<'a>>();
        return R::from(BaseIndex::from_usize(self.data.len()));
    }
}
