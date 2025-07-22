use std::{borrow::Borrow, cell::Cell, fmt, marker::PhantomData, ops};

use super::*;
// type Ptr<'a, T: ?Sized> = BaseIndex<'a, T>;

pub trait SubspaceResident<'a, HeapRepr: ?Sized> {
    type Space: Subspace<'a, HeapRepr, Self>;
    fn subspace_for(heap: &Heap) -> &Self::Space;
    fn subspace_for_mut(heap: &mut Heap) -> &mut Self::Space;
}
pub(crate) trait HeapIndexable {
    fn get_index(self) -> usize;
}
pub(crate) trait SubspaceIndex<'a, T: ?Sized>: From<BaseIndex<'a, T>> + HeapIndexable {
    /// # Do not use this
    /// This is only for Value discriminant creation.
    const _DEF: Self;
    // const fn _def() -> Self {
    //     Self(BaseIndex::from_u32_index(0))
    // }
    // fn get_index(self) -> usize;
    //  {
    //     self.0.into_index()
    // }
    // fn id(self) -> BaseIndex<'a, T>;
    // fn get_index(self) -> usize {

    // }
}

// pub trait IsoSubspaceResident {
//     type Data<'a>: Bindable<Of<'a> = Self::Data<'a>>;
// }
pub trait IsoSubspaceResident {
    type Key<'a>: SubspaceIndex<'a, Self>;
}

macro_rules! declare_subspace_resident {
    (iso; struct $Nominal:ident, $Data:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $Nominal<'a>(BaseIndex<'a, $Data<'static>>);

        impl<'a> From<BaseIndex<'a, $Data<'static>>> for $Nominal<'a> {
            fn from(value: BaseIndex<'a, $Data<'static>>) -> Self {
                $Nominal(value)
            }
        }

        impl crate::heap::HeapIndexable for $Nominal<'_> {
            #[inline]
            fn get_index(self) -> usize {
                self.0.into_index()
            }
        }

        impl<'a> crate::heap::SubspaceIndex<'a, $Data<'static>> for $Nominal<'a> {
            const _DEF: Self = Self(BaseIndex::from_u32_index(0));
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

        impl crate::heap::IsoSubspaceResident for $Data<'static> {
            type Key<'a> = $Nominal<'a>;
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

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// TODO: do not rely on len(). subspace will eventually store data across
    /// various blocks to avoid massive re-allocations
    #[inline]
    pub(super) fn len(&self) -> usize {
        self.data.len()
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
    T: HeapIndexable,
{
    type Output = D;
    fn index(&self, index: T) -> &Self::Output {
        self.data
            .get(index.get_index())
            .expect("subspace index out of bounds")
            .as_ref()
            .expect("subspace slot is empty")
    }
}
impl<T, D> ops::IndexMut<T> for IsoSubspace<T, D>
where
    T: HeapIndexable
{
    fn index_mut(&mut self, index: T) -> &mut Self::Output {
        self.data
            .get_mut(index.get_index())
            .expect("subspace index out of bounds")
            .as_mut()
            .expect("subspace slot is empty")
    }
}

impl<'a, R> Subspace<'a, R, R::Key<'a>> for IsoSubspace<R::Key<'a>, R>
where
    // R: IsoSubspaceResident + From<BaseIndex<'a, R::Data<'static>>>,
    R: IsoSubspaceResident,
{
    fn alloc(&'a mut self, data: R) -> R::Key<'a> {
        // SAFETY: this is not safe. fixme.
        // let d: R::Data<'static> = unsafe { std::mem::transmute(data) };
        self.data.push(Some(data));
        self.alloc_count += core::mem::size_of::<R>();
        return R::Key::from(BaseIndex::from_usize(self.data.len()));
    }
}

impl<'a, T, D> IsoSubspace<T, D>
where
    T: SubspaceIndex<'a, D>,
    D: HeapMarkAndSweep,
{
    pub(crate) fn mark<M>(
        &self, //
        marks: M,
        bits: &mut [bool],
        queues: &mut WorkQueues,
    ) where
        M: IntoIterator<Item = T>,
    {
        marks.into_iter().for_each(|idx| {
            let index = idx.get_index();
            if let Some(marked) = bits.get_mut(index) {
                if *marked {
                    // Already marked, ignore
                    return;
                }
                *marked = true;
                self.data.get(index).mark_values(queues);
            }
        });
    }
    pub(crate) fn sweep(&mut self, compactions: &CompactionLists, bits: &[bool]) {
        assert_eq!(self.data.len(), bits.len());
        let mut iter = bits.iter();
        self.data.retain_mut(|item| {
            let do_retain = iter.next().unwrap();
            if *do_retain {
                item.sweep_values(compactions);
                true
            } else {
                false
            }
        });
    }
}

impl<T, D> fmt::Debug for IsoSubspace<T, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsoSubspace")
            .field("name", &self.name)
            .field("alloc_count", &self.alloc_count)
            .field("data", &"<omitted>")
            .finish()
    }
}
