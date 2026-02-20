// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use hashbrown::HashTable;

use crate::{
    ecmascript::{Agent, PropertyKey},
    engine::{
        Bindable, HeapRootCollectionData, NoGcScope, ScopableCollection, ScopedCollection,
        bindable_handle,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

/// An unordered set of PropertyKeys.
#[derive(Clone, Default)]
#[repr(transparent)]
pub struct PropertyKeySet<'a>(HashTable<PropertyKey<'a>>);

impl core::fmt::Debug for PropertyKeySet<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> PropertyKeySet<'a> {
    pub fn new(_: NoGcScope<'a, '_>) -> Self {
        Self(HashTable::new())
    }

    pub fn with_capacity(capacity: usize, _: NoGcScope<'a, '_>) -> Self {
        Self(HashTable::with_capacity(capacity))
    }

    /// Insert a PropertyKey into the set.
    ///
    /// The insertion might trigger a resize of the underlying hash table,
    /// requiring rehashing of some or all previous elements. Hence the
    /// PropertyKeyHeap parameter is needed.
    pub fn insert(&mut self, agent: &Agent, value: PropertyKey) -> bool {
        let hash = value.heap_hash(agent);
        let entry = self.0.entry(hash, |p| *p == value, |p| p.heap_hash(agent));
        match entry {
            hashbrown::hash_table::Entry::Occupied(_) => false,
            hashbrown::hash_table::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(value);
                true
            }
        }
    }

    /// Returns `true` if the set contains a PropertyKey.
    pub fn contains(&self, agent: &Agent, value: PropertyKey) -> bool {
        let hash = value.heap_hash(agent);
        self.0.find(hash, |p| *p == value).is_some()
    }
}

impl ScopableCollection for PropertyKeySet<'_> {
    fn scope<'scope>(
        self,
        agent: &Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> ScopedCollection<'scope, Self::Of<'static>> {
        ScopedCollection::new(agent, self,gc)
    }
}

impl ScopedCollection<'_, PropertyKeySet<'static>> {
    /// Insert a PropertyKey into the scoped set.
    ///
    /// The insertion might trigger a resize of the underlying hash table,
    /// requiring rehashing of some or all previous elements. Hence the
    /// PropertyKeyHeap parameter is needed.
    pub fn insert(&mut self, agent: &Agent, value: PropertyKey) -> bool {
        let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
        let Some(stack_slot) = stack_ref_collections.get_mut(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollectionData::PropertyKeySet(property_key_set) = stack_slot else {
            unreachable!()
        };
        property_key_set.insert(agent, value)
    }

    /// Returns `true` if the scoped set contains a PropertyKey.
    pub fn contains(&self, agent: &Agent, value: PropertyKey) -> bool {
        let stack_ref_collections = agent.stack_ref_collections.borrow();
        let Some(stack_slot) = stack_ref_collections.get(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollectionData::PropertyKeySet(property_key_set) = stack_slot else {
            unreachable!()
        };
        property_key_set.contains(agent, value)
    }
}

bindable_handle!(PropertyKeySet);

impl HeapMarkAndSweep for PropertyKeySet<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        self.0.iter().for_each(|p| p.mark_values(queues));
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        self.0.iter_mut().for_each(|p| p.sweep_values(compactions));
    }
}
