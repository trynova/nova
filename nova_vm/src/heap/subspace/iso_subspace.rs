use super::{HeapIndexable, Subspace};
use std::{fmt, ops};
// use crate::{engine::context::Bindable, heap::indexes::BaseIndex};
use crate::heap::*;

pub struct IsoSubspace<D> {
    name: &'static str,
    alloc_count: usize,
    data: Vec<Option<D>>,
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
    T: SubspaceResident,
{
    pub fn get(&self, key: T::Key<'_>) -> Option<&T> {
        self.data.get(key.get_index()).and_then(Option::as_ref)
    }
    pub fn get_mut(&mut self, key: T::Key<'_>) -> Option<&mut T> {
        self.data.get_mut(key.get_index()).and_then(Option::as_mut)
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

impl<T> ops::Index<T::Key<'_>> for IsoSubspace<T>
where
    T: SubspaceResident,
{
    type Output = T;

    fn index(&self, index: T::Key<'_>) -> &Self::Output {
        let i = index.get_index();
        self.data
            .get(i)
            .unwrap_or_else(|| panic!("subspace {}: index out of bounds", self.name))
            .as_ref()
            .unwrap_or_else(|| panic!("subspace {}: slot {i} is empty", self.name))
    }
}

impl<T> ops::IndexMut<T::Key<'_>> for IsoSubspace<T>
where
    T: SubspaceResident,
{
    fn index_mut(&mut self, index: T::Key<'_>) -> &mut Self::Output {
        let i = index.get_index();
        self.data
            .get_mut(i)
            .unwrap_or_else(|| panic!("subspace {}: index out of bounds", self.name))
            .as_mut()
            .unwrap_or_else(|| panic!("subspace {}: slot {i} is empty", self.name))
    }
}

impl<T> Subspace<T> for IsoSubspace<T>
where
    T: SubspaceResident,
{
    fn alloc<'a>(&mut self, data: T::Bound<'a>) -> T::Key<'a> {
        self.data.push(Some(data.unbind()));
        self.alloc_count += core::mem::size_of::<T>();
        T::Key::from(BaseIndex::from_usize(self.data.len()))
    }
}

impl<T> IsoSubspace<T>
where
    T: SubspaceResident,
{
    pub(crate) fn reserve_intrinsic(&mut self) -> T::Key<'static> {
        self.data.push(None);
        // note: not from_index b/c len is now +1
        T::Key::from(BaseIndex::from_usize(self.len()))
    }
    pub(crate) fn create<'a>(&mut self, data: T::Bound<'a>) -> T::Key<'a> {
        let d: T = unsafe { core::mem::transmute(data.unbind()) };

        self.data.push(Some(d));
        self.alloc_count += core::mem::size_of::<T>();
        T::Key::from(BaseIndex::from_usize(self.data.len()))
    }
}

impl<T> IsoSubspace<T>
where
    T: SubspaceResident + HeapMarkAndSweep,
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
        let items_before = self.data.len();
        self.data.retain_mut(|item| {
            let do_retain = iter.next().unwrap();
            if *do_retain {
                item.sweep_values(compactions);
                true
            } else {
                false
            }
        });
        let items_dropped = items_before.saturating_sub(self.data.len());
        self.alloc_count -= items_dropped * core::mem::size_of::<T>()
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
