use std::{borrow::Borrow, cell::Cell, fmt, marker::PhantomData, ops};
use super::{SubspaceResident, Subspace, SubspaceIndex, HeapIndexable};
// use crate::{engine::context::Bindable, heap::indexes::BaseIndex};
use crate::heap::*;

pub struct IsoSubspace<T, D> {
    name: &'static str,
    alloc_count: usize,
    data: Vec<Option<D>>,
    _marker: PhantomData<T>,
}

pub trait IsoSubspaceResident : Bindable {
    type Key<'a>: SubspaceIndex<'a, Self>;
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
    T: HeapIndexable
{
    pub fn get(&self, key: T) -> Option<&D> {
        self.data.get(key.get_index()).map(Option::as_ref).flatten()
    }
    pub fn get_mut(&mut self, key: T) -> Option<&mut D> {
        self.data.get_mut(key.get_index()).map(Option::as_mut).flatten()
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

impl<R> ops::Index<R::Key<'_>> for IsoSubspace<R::Key<'_>, R>
where
    R: IsoSubspaceResident,
{
    type Output = R;

    fn index(&self, index: R::Key<'_>) -> &Self::Output {
        self.data
            .get(index.get_index())
            .expect("subspace index out of bounds")
            .as_ref()
            .expect("subspace slot is empty")
    }
}

impl<R> ops::IndexMut<R::Key<'_>> for IsoSubspace<R::Key<'_>, R>
where
    R: IsoSubspaceResident,
{
    fn index_mut(&mut self, index: R::Key<'_>) -> &mut Self::Output {
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

impl<'a, R> IsoSubspace<R::Key<'static>, R> where
    R: IsoSubspaceResident + Bindable,
    R::Of<'static> : Into<R>
{
    pub fn create(&mut self, data: R) -> R::Key<'a> {
        self.data.push(Some(data.unbind().into()));
        self.alloc_count += core::mem::size_of::<R>();
        return R::Key::from(BaseIndex::from_usize(self.data.len()));
    }
}

impl<'a, T, D> IsoSubspace<T, D>
where
    T: HeapIndexable, //SubspaceIndex<'a, D>,
    D: HeapMarkAndSweep
    // D: HeapMarkAndSweep + Bindable,
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
