use super::{HeapIndexable, Subspace, SubspaceIndex, SubspaceResident};
use std::{borrow::Borrow, cell::Cell, fmt, marker::PhantomData, ops};
// use crate::{engine::context::Bindable, heap::indexes::BaseIndex};
use crate::heap::*;

pub struct IsoSubspace<D> {
    name: &'static str,
    alloc_count: usize,
    data: Vec<Option<D>>,
    // _marker: PhantomData<T>,
}

pub trait IsoSubspaceResident: Bindable {
    type Key<'a>: SubspaceIndex<'a, Self>;
    type X<'a>: Bindable<Of<'static> = Self>;
}

impl<D> IsoSubspace<D> {
    fn new(name: &'static str) -> Self {
        Self::with_capacity(name, 0)
    }

    pub fn with_capacity(name: &'static str, capacity: usize) -> Self {
        Self {
            name,
            alloc_count: 0,
            data: Vec::with_capacity(capacity),
            // _marker: PhantomData,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// TODO: do not rely on len(). subspace will eventually store data across
    /// various blocks to avoid massive re-allocations
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }
}

impl<T> IsoSubspace<T>
where
    T: IsoSubspaceResident,
{
    pub fn get(&self, key: T::Key<'_>) -> Option<&T> {
        self.data.get(key.get_index()).map(Option::as_ref).flatten()
    }
    pub fn get_mut(&mut self, key: T::Key<'_>) -> Option<&mut T> {
        self.data
            .get_mut(key.get_index())
            .map(Option::as_mut)
            .flatten()
    }
    pub fn slot(&self, key: T::Key<'_>) -> &Option<T> {
        self.data.get(key.get_index()).expect("Slot out of bounds")
    }
    pub fn slot_mut(&mut self, key: T::Key<'_>) -> &mut Option<T> {
        self.data
            .get_mut(key.get_index())
            .expect("Slot out of bounds")
    }
}

// impl<T, D> ops::Index<T> for IsoSubspace<T, D>
// where
//     T: HeapIndexable,
// {
//     type Output = D;
//     fn index(&self, index: T) -> &Self::Output {
//         self.data
//             .get(index.get_index())
//             .expect("subspace index out of bounds")
//             .as_ref()
//             .expect("subspace slot is empty")
//     }
// }

// impl<T, D> ops::IndexMut<T> for IsoSubspace<T, D>
// where
//     T: HeapIndexable,
// {
//     fn index_mut(&mut self, index: T) -> &mut Self::Output {
//         self.data
//             .get_mut(index.get_index())
//             .expect("subspace index out of bounds")
//             .as_mut()
//             .expect("subspace slot is empty")
//     }
// }

impl<T> ops::Index<T::Key<'_>> for IsoSubspace<T>
where
    T: IsoSubspaceResident,
{
    type Output = T;

    fn index(&self, index: T::Key<'_>) -> &Self::Output {
        self.data
            .get(index.get_index())
            .expect("subspace index out of bounds")
            .as_ref()
            .expect("subspace slot is empty")
    }
}

impl<T> ops::IndexMut<T::Key<'_>> for IsoSubspace<T>
where
    T: IsoSubspaceResident,
{
    fn index_mut(&mut self, index: T::Key<'_>) -> &mut Self::Output {
        self.data
            .get_mut(index.get_index())
            .expect("subspace index out of bounds")
            .as_mut()
            .expect("subspace slot is empty")
    }
}

impl<'a, T> Subspace<'a, T, T::Key<'a>> for IsoSubspace<T>
where
    // R: IsoSubspaceResident + From<BaseIndex<'a, R::Data<'static>>>,
    T: IsoSubspaceResident,
{
    fn alloc(&'a mut self, data: T) -> T::Key<'a> {
        // SAFETY: this is not safe. fixme.
        // let d: R::Data<'static> = unsafe { std::mem::transmute(data) };
        self.data.push(Some(data));
        self.alloc_count += core::mem::size_of::<T>();
        return T::Key::from(BaseIndex::from_usize(self.data.len()));
    }
}

impl<T> IsoSubspace<T>
where
    T: IsoSubspaceResident,
{
    pub (crate) fn reserve_intrinsic(&mut self) -> T::Key<'static> {
        self.data.push(None);
        // note: not from_index b/c len is now +1
        return T::Key::from(BaseIndex::from_usize(self.len()))

    }
    pub(crate) fn create<'a>(&mut self, data: T::X<'a>) -> T::Key<'a>
        // for<'a> U: IsoSubspaceResident<Key<'a> = T::Key<'a>, Of<'a> = T::Of<'a>>,
    {
        let d: T = unsafe { core::mem::transmute(data.unbind()) };

        self.data.push(Some(d));
        self.alloc_count += core::mem::size_of::<T>();
        return T::Key::from(BaseIndex::from_usize(self.data.len()));
    }
    // fn create<U>(&mut self, data: U) -> T::Key<'_>
    // where
    //     for<'a> U: IsoSubspaceResident<Key<'a> = T::Key<'a>, Of<'a> = T::Of<'a>>,
    // {
}

// impl<'a, /* static */ T, /* gc bound*/ U> IsoSubspace<T>
// where
//     T: IsoSubspaceResident<Of<'a> = U>,
//     U: IsoSubspaceResident<Of<'static> = T>,
//     // T::Of<'static>
// {
//     pub fn create(&mut self, data: U) -> T::Key<'a> {
//         self.data.push(Some(data.unbind()));
//         self.alloc_count += core::mem::size_of::<T>();
//         return T::Key::from(BaseIndex::from_usize(self.data.len()));
//     }
// }

impl<T> IsoSubspace<T>
where
    T: IsoSubspaceResident + HeapMarkAndSweep,
{
    pub(crate) fn mark<'a, M>(
        &'a self, //
        marks: M,
        bits: &mut [bool],
        queues: &mut WorkQueues,
    ) where
        M: IntoIterator<Item = T::Key<'a>>,
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

impl<T> fmt::Debug for IsoSubspace<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IsoSubspace")
            .field("name", &self.name)
            .field("alloc_count", &self.alloc_count)
            .field("data", &"<omitted>")
            .finish()
    }
}
