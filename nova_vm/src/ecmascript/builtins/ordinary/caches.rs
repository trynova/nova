// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{marker::PhantomData, num::NonZeroU32, ops::Neg};

use hashbrown::{HashTable, hash_table::Entry};

use crate::{
    ecmascript::{
        execution::Agent,
        types::{Object, PropertyKey},
    },
    engine::context::{Bindable, GcToken, NoGcScope},
    heap::{
        CompactionLists, HeapMarkAndSweep, HeapSweepWeakReference, PropertyKeyHeap, WeakReference,
        WorkQueues, sweep_heap_vector_values,
    },
};

use super::shape::ObjectShape;

/// Heap structure holding all property lookup caches, cache-caches, and other
/// related features.
#[derive(Debug)]
pub(crate) struct Caches<'a> {
    property_lookup_cache_lookup_table:
        HashTable<(PropertyKey<'a>, WeakReference<PropertyLookupCache<'a>>)>,
    property_lookup_caches: Vec<PropertyLookupCacheRecord<'a>>,
    property_lookup_cache_prototypes: Vec<PropertyLookupCacheRecordPrototypes<'a>>,
    property_lookup_cache_stack: Vec<PropertyLookupCache<'a>>,
    current_property_lookup_cache: Option<PropertyLookupCache<'a>>,
}

impl Caches<'_> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            property_lookup_cache_lookup_table: HashTable::with_capacity(capacity),
            property_lookup_caches: Vec::with_capacity(capacity),
            property_lookup_cache_prototypes: Vec::with_capacity(capacity),
            property_lookup_cache_stack: Vec::with_capacity(64),
            current_property_lookup_cache: None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.property_lookup_caches.len()
    }
}

impl Caches<'static> {
    pub(crate) fn mark_cache(&self, index: usize, queues: &mut WorkQueues) {
        self.property_lookup_caches[index].mark_values(queues);
        self.property_lookup_cache_prototypes[index].mark_values(queues);
    }

    pub(crate) fn sweep_cache(&mut self, compactions: &CompactionLists, bits: &[bool]) {
        sweep_heap_vector_values(&mut self.property_lookup_caches, compactions, bits);
        sweep_heap_vector_values(
            &mut self.property_lookup_cache_prototypes,
            compactions,
            bits,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct PropertyLookupCache<'a>(NonZeroU32, PhantomData<&'a GcToken>);

impl<'a> PropertyLookupCache<'a> {
    pub(crate) fn new(
        agent: &mut Agent,
        shape: ObjectShape<'a>,
        key: PropertyKey<'a>,
        offset: usize,
    ) -> Option<Self> {
        let index = PropertyLookupCacheIndex::new(offset)?;
        let hash = key.heap_hash(agent);
        let caches = &mut agent.heap.caches;
        let property_key_heap = PropertyKeyHeap::new(&agent.heap.strings, &agent.heap.symbols);
        let entry = caches.property_lookup_cache_lookup_table.entry(
            hash,
            |(k, _)| *k == key,
            |(k, _)| k.heap_hash(&property_key_heap),
        );
        let WeakReference(cache_key) = match entry {
            Entry::Occupied(e) => e.get().1,
            Entry::Vacant(e) => {
                caches
                    .property_lookup_caches
                    .push(PropertyLookupCacheRecord::from_shape_and_index(shape, index).unbind());
                caches
                    .property_lookup_cache_prototypes
                    .push(PropertyLookupCacheRecordPrototypes::new().unbind());
                let cache = PropertyLookupCache::last(&caches.property_lookup_caches);
                e.insert((key.unbind(), WeakReference(cache.unbind())));
                return Some(cache);
            }
        };
        // Note: iterate the lookup cache record linked list, but always return
        // the first link in the chain.
        let mut cache_chain = cache_key;
        loop {
            let len = caches.property_lookup_caches.len();
            let (cache_record, _) = cache_chain.get_mut(caches);
            if cache_record.find(shape).is_some() || cache_record.insert(shape, index) {
                return Some(cache_key);
            }
            let Some(next_cache) = cache_record.next_cache else {
                let cache = PropertyLookupCache::from_index(len);
                cache_record.next_cache = Some(cache);
                caches
                    .property_lookup_caches
                    .push(PropertyLookupCacheRecord::from_shape_and_index(shape, index).unbind());
                caches
                    .property_lookup_cache_prototypes
                    .push(PropertyLookupCacheRecordPrototypes::new().unbind());
                return Some(cache_key);
            };
            cache_chain = next_cache;
        }
    }

    #[inline(always)]
    fn from_index(index: usize) -> Self {
        Self(
            NonZeroU32::new(u32::try_from(index).unwrap().checked_add(1).unwrap()).unwrap(),
            PhantomData,
        )
    }

    #[inline(always)]
    fn last(caches: &[PropertyLookupCacheRecord<'a>]) -> Self {
        Self(
            NonZeroU32::new(u32::try_from(caches.len()).unwrap()).unwrap(),
            PhantomData,
        )
    }

    #[inline(always)]
    pub(crate) fn get_index(self) -> usize {
        self.0.get() as usize
    }

    #[inline(always)]
    fn get<'c>(
        self,
        caches: &'c Caches<'a>,
    ) -> (
        &'c PropertyLookupCacheRecord<'a>,
        &'c PropertyLookupCacheRecordPrototypes<'a>,
    ) {
        let index = self.get_index();
        (
            &caches.property_lookup_caches[index],
            &caches.property_lookup_cache_prototypes[index],
        )
    }

    #[inline(always)]
    fn get_mut<'c>(
        self,
        caches: &'c mut Caches<'static>,
    ) -> (
        &'c mut PropertyLookupCacheRecord<'static>,
        &'c mut PropertyLookupCacheRecordPrototypes<'static>,
    ) {
        let index = self.get_index();
        (
            &mut caches.property_lookup_caches[index],
            &mut caches.property_lookup_cache_prototypes[index],
        )
    }
}

// SAFETY: Properly implemented as a lifetime transmute.
unsafe impl Bindable for PropertyLookupCache<'_> {
    type Of<'a> = PropertyLookupCache<'a>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

const N: usize = 4;

#[derive(Debug)]
pub(crate) struct PropertyLookupCacheRecord<'a> {
    shapes: [Option<ObjectShape<'a>>; N],
    indexes: [PropertyLookupCacheIndex; N],
    next_cache: Option<PropertyLookupCache<'a>>,
}

impl<'a> PropertyLookupCacheRecord<'a> {
    pub(crate) const fn new() -> Self {
        Self {
            shapes: [None; N],
            indexes: [PropertyLookupCacheIndex(0); N],
            next_cache: None,
        }
    }

    pub(crate) fn from_shape_and_index(
        shape: ObjectShape<'a>,
        index: PropertyLookupCacheIndex,
    ) -> Self {
        Self {
            shapes: [Some(shape), None, None, None],
            indexes: [
                index,
                PropertyLookupCacheIndex(0),
                PropertyLookupCacheIndex(0),
                PropertyLookupCacheIndex(0),
            ],
            next_cache: None,
        }
    }

    /// Find the property lookup cache for the given Object Shape.
    pub(crate) fn find(&self, shape: ObjectShape<'a>) -> Option<PropertyLookupCacheIndex> {
        self.shapes
            .iter()
            .enumerate()
            .find(|(_, s)| **s == Some(shape))
            .map(|(i, _)| self.indexes[i])
    }

    /// Insert the given Object Shape and lookup cache index into the property
    /// lookup cache record. Returns false if the record is full.
    pub(crate) fn insert(&mut self, shape: ObjectShape, index: PropertyLookupCacheIndex) -> bool {
        if let Some((i, slot)) = self
            .shapes
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.is_none())
        {
            *slot = Some(shape.unbind());
            self.indexes[i] = index;
            true
        } else {
            false
        }
    }
}

// SAFETY: Properly implemented as a lifetime transmute.
unsafe impl Bindable for PropertyLookupCacheRecord<'_> {
    type Of<'a> = PropertyLookupCacheRecord<'a>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct PropertyLookupCacheIndex(i16);

impl PropertyLookupCacheIndex {
    /// Create a new property lookup cache index from an offset.
    ///
    /// Returns None if the offset is beyond supported limits.
    #[inline(always)]
    pub(crate) fn new(offset: usize) -> Option<Self> {
        Some(Self(i16::try_from(offset).ok()?))
    }

    /// Convert a property lookup into a prototype property lookup.
    pub(crate) fn into_prototype_lookup(self) -> Self {
        debug_assert!(!self.is_prototype_lookup());
        Self(self.0.neg())
    }

    /// Returns true if the property was found on the Object Shape's prototype.
    #[inline(always)]
    pub(crate) fn is_prototype_lookup(self) -> bool {
        self.0.is_negative()
    }

    /// Returns the offset that the property was found at.
    #[inline(always)]
    pub(crate) fn get_property_offset(self) -> usize {
        self.0.abs() as usize
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct PropertyLookupCacheRecordPrototypes<'a> {
    prototypes: [Option<Object<'a>>; N],
}

impl<'a> PropertyLookupCacheRecordPrototypes<'a> {
    pub(crate) const fn new() -> Self {
        Self {
            prototypes: [None; N],
        }
    }

    /// Insert the given prototype into the property lookup cache record
    /// prototypes.
    ///
    /// ## Panics
    ///
    /// If the index is over the bounds, or if the prototype at the given
    /// index is already set.
    pub(crate) fn insert(&mut self, index: usize, prototype: Object) {
        let slot = &mut self.prototypes[index];
        let previous = slot.replace(prototype.unbind());
        debug_assert!(previous.is_none());
    }
}

// SAFETY: Properly implemented as a lifetime transmute.
unsafe impl Bindable for PropertyLookupCacheRecordPrototypes<'_> {
    type Of<'a> = PropertyLookupCacheRecordPrototypes<'a>;

    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    fn bind<'a>(self, _: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for Caches<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            property_lookup_cache_lookup_table,
            // Note: caches are not marked unconditionally; the heap_gc function
            // will call mark_cache as needed.
            property_lookup_caches: _,
            property_lookup_cache_prototypes: _,
            property_lookup_cache_stack,
            current_property_lookup_cache,
        } = self;
        // Note: we do not mark the caches in the lookup table; they're weakly
        // held.
        for (key, _) in property_lookup_cache_lookup_table.iter() {
            key.mark_values(queues);
        }
        property_lookup_cache_stack.mark_values(queues);
        current_property_lookup_cache.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            property_lookup_cache_lookup_table,
            // Note: caches are not sweeped here; the heap_gc function
            // will call sweep_cache.
            property_lookup_caches: _,
            property_lookup_cache_prototypes: _,
            property_lookup_cache_stack,
            current_property_lookup_cache,
        } = self;
        property_lookup_cache_lookup_table.retain(|(key, cache)| {
            let Some(new_cache) = cache.sweep_weak_reference(compactions) else {
                return false;
            };
            key.sweep_values(compactions);
            *cache = new_cache;
            true
        });
        property_lookup_cache_stack.sweep_values(compactions);
        current_property_lookup_cache.sweep_values(compactions);
    }
}

impl HeapSweepWeakReference for PropertyLookupCache<'_> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .caches
            .shift_weak_non_zero_u32_index(self.0)
            .map(|i| Self(i, PhantomData))
    }
}

impl HeapMarkAndSweep for PropertyLookupCache<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.caches.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.caches.shift_non_zero_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for PropertyLookupCacheRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            shapes,
            indexes: _,
            next_cache,
        } = self;
        shapes.as_slice().mark_values(queues);
        next_cache.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            shapes,
            indexes: _,
            next_cache,
        } = self;
        shapes.as_mut_slice().sweep_values(compactions);
        next_cache.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for PropertyLookupCacheRecordPrototypes<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self { prototypes } = self;
        prototypes.as_slice().mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self { prototypes } = self;
        prototypes.as_mut_slice().sweep_values(compactions);
    }
}
