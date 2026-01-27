// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::hash::Hasher;

use ahash::AHasher;
use hashbrown::{HashTable, hash_table::Entry};

use crate::{
    ecmascript::{PropertyKey, Value, execution::Agent},
    engine::{
        ScopableCollection, ScopedCollection,
        Bindable, NoGcScope, bindable_handle,
        HeapRootCollectionData,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

pub struct KeyedGroup<'a> {
    // TODO: Use a SoA vector for keys and values.
    keys: Vec<Value<'a>>,
    values: Vec<Vec<Value<'a>>>,
    hash_table: HashTable<u32>,
}

impl core::fmt::Debug for KeyedGroup<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "KeyedGroup")?;
        f.debug_map()
            .entries(self.keys.iter().zip(self.values.iter()))
            .finish()
    }
}

impl<'a> KeyedGroup<'a> {
    pub(crate) fn new(_: NoGcScope<'a, '_>) -> Box<Self> {
        Box::new(Self {
            keys: Vec::new(),
            values: Vec::new(),
            hash_table: HashTable::new(),
        })
    }

    pub(crate) fn len(&self) -> usize {
        debug_assert!(
            self.keys.len() == self.values.len() && self.keys.len() == self.hash_table.len()
        );
        self.keys.len()
    }

    /// Add a value to a keyed group using the COLLECTION key coercion.
    ///
    /// It is not allowed to mix PROPERTY and COLLECTION key coercions in the
    /// same keyed group.
    pub(crate) fn add_collection_keyed_value(&mut self, agent: &Agent, key: Value, value: Value) {
        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            value.hash(agent, &mut hasher);
            hasher.finish()
        };
        let hash = hasher(key);
        let entry = self.hash_table.entry(
            hash,
            |idx| self.keys[*idx as usize] == key,
            |idx| {
                let key = self.keys[*idx as usize];
                hasher(key)
            },
        );
        match entry {
            Entry::Occupied(occupied) => {
                let idx = *occupied.get() as usize;
                self.values[idx].push(value.unbind());
            }
            Entry::Vacant(vacant) => {
                let idx = u32::try_from(self.keys.len()).expect("Keyed group overflowed");
                self.keys.push(key.unbind());
                self.values.push(vec![value.unbind()]);
                vacant.insert(idx);
            }
        }
    }

    /// Add a value to a keyed group using the PROPERTY key coercion.
    ///
    /// It is not allowed to mix PROPERTY and COLLECTION key coercions in the
    /// same keyed group.
    pub(crate) fn add_property_keyed_value(
        &mut self,
        agent: &Agent,
        key: PropertyKey,
        value: Value,
    ) {
        let hash = key.heap_hash(agent);
        // SAFETY: Caller is using this KeyedGroup with the PROPERTY key
        // coercion.
        let key = unsafe { key.into_value_unchecked() };
        let entry = self.hash_table.entry(
            hash,
            |idx| self.keys[*idx as usize] == key,
            |idx| {
                let key = self.keys[*idx as usize];
                // SAFETY: Caller is using this KeyedGroup with the PROPERTY
                // key coercion.
                let key = unsafe { PropertyKey::from_value_unchecked(key) };
                key.heap_hash(agent)
            },
        );
        match entry {
            Entry::Occupied(occupied) => {
                let idx = *occupied.get() as usize;
                self.values[idx].push(value.unbind());
            }
            Entry::Vacant(vacant) => {
                let idx = u32::try_from(self.keys.len()).expect("Keyed group overflowed");
                self.keys.push(key.unbind());
                self.values.push(vec![value.unbind()]);
                vacant.insert(idx);
            }
        }
    }

    fn insert_to_hash_table_unique(
        hash_table: &mut HashTable<u32>,
        keys: &[Value<'static>],
        key: Value<'static>,
        index: u32,
    ) {
        let hasher = |value: Value| {
            let mut hasher = AHasher::default();
            value.try_hash(&mut hasher).unwrap();
            hasher.finish()
        };
        let hash = hasher(key);
        let entry = hash_table.entry(
            hash,
            |idx| {
                // The caller has promised us that the key is unique.
                let indexes_equal = *idx == index;
                let values_equal = keys[*idx as usize] == key;
                assert!(!indexes_equal || !values_equal);
                false
            },
            |idx| {
                let key = keys[*idx as usize];
                hasher(key)
            },
        );
        match entry {
            Entry::Occupied(_) => {
                unreachable!()
            }
            Entry::Vacant(vacant) => {
                vacant.insert(index);
            }
        }
    }

    /// Iterate over the keyed groups using the PROPERTY key coercion.
    pub(crate) fn into_property_keyed_iter(
        self,
    ) -> impl Iterator<Item = (PropertyKey<'a>, Vec<Value<'a>>)> {
        debug_assert_eq!(self.keys.len(), self.values.len());
        self.keys
            .into_iter()
            .map(|v| unsafe { PropertyKey::from_value_unchecked(v) })
            .zip(self.values)
    }

    /// Iterate over the keyed groups using the COLLECTION key coercion.
    pub(crate) fn into_collection_keyed_iter(
        self,
    ) -> impl Iterator<Item = (Value<'a>, Vec<Value<'a>>)> {
        debug_assert_eq!(self.keys.len(), self.values.len());
        self.keys.into_iter().zip(self.values)
    }
}

impl ScopableCollection for Box<KeyedGroup<'_>> {
    fn scope<'scope>(
        self,
        agent: &Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> ScopedCollection<'scope, Self::Of<'static>> {
        ScopedCollection::new(agent, self.unbind(), gc)
    }
}

impl ScopedCollection<'_, Box<KeyedGroup<'static>>> {
    /// Add a value to a scoped keyed group using the COLLECTION key coercion.
    ///
    /// It is not allowed to mix PROPERTY and COLLECTION key coercions in the
    /// same keyed group.
    pub(crate) fn add_collection_keyed_value(&mut self, agent: &Agent, key: Value, value: Value) {
        let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
        let Some(stack_slot) = stack_ref_collections.get_mut(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollectionData::KeyedGroup(keyed_group) = stack_slot else {
            unreachable!()
        };
        keyed_group.add_collection_keyed_value(agent, key, value);
    }

    /// Add a value to a keyed group using the PROPERTY key coercion.
    ///
    /// It is not allowed to mix PROPERTY and COLLECTION key coercions in the
    /// same keyed group.
    pub(crate) fn add_property_keyed_value(
        &mut self,
        agent: &Agent,
        key: PropertyKey,
        value: Value,
    ) {
        let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
        let Some(stack_slot) = stack_ref_collections.get_mut(self.inner as usize) else {
            unreachable!();
        };
        let HeapRootCollectionData::KeyedGroup(keyed_group) = stack_slot else {
            unreachable!()
        };
        keyed_group.add_property_keyed_value(agent, key, value);
    }
}

bindable_handle!(KeyedGroup);

fn value_needs_rehash(v: &Value<'static>, compactions: &CompactionLists) -> bool {
    // Note: Symbols currently do not have static hashes; this will
    // eventually need to change.
    if v.is_symbol() || v.is_object() {
        let mut new_v = *v;
        new_v.sweep_values(compactions);
        if *v != new_v {
            // Hash does change. This happens when the object is not
            // shifted by the GC.
            true
        } else {
            false
        }
    } else {
        // Non-symbol primitives have a static hash.
        false
    }
}

impl HeapMarkAndSweep for KeyedGroup<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            keys,
            values,
            hash_table: _,
        } = self;
        keys.mark_values(queues);
        for vec in values.iter() {
            vec.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            keys,
            values,
            hash_table,
        } = self;
        if keys.iter().any(|v| value_needs_rehash(v, compactions)) {
            if keys
                .iter()
                .any(|v| matches!(v, Value::String(_) | Value::Number(_) | Value::BigInt(_)))
            {
                // The group needs rehashing of both primitive, heap-stored
                // keys and object keys. This can lead to a situation where the
                // entire group needs to be rehashed and we cannot do so
                // because the primitive heap data cannot be accessed during
                // GC. We'll try to avoid this as best we can by first deleting
                // all object (and symbol) keys that do need rehashing.
                let rehash_indexes = keys
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| value_needs_rehash(v, compactions))
                    .map(|(i, _)| i as u32)
                    .collect::<Vec<u32>>();
                hash_table.retain(|idx| !rehash_indexes.contains(idx));
                keys.sweep_values(compactions);
                for vec in values.iter_mut() {
                    vec.sweep_values(compactions);
                }
                for idx in rehash_indexes.into_iter() {
                    let key = keys[idx as usize];
                    KeyedGroup::insert_to_hash_table_unique(hash_table, keys, key, idx);
                }
            } else {
                // All keys can be rehashed in place.
                hash_table.clear();
                keys.sweep_values(compactions);
                for vec in values.iter_mut() {
                    vec.sweep_values(compactions);
                }
                for (index, key) in keys.iter().enumerate() {
                    let index = index as u32;
                    let key = *key;
                    KeyedGroup::insert_to_hash_table_unique(hash_table, keys, key, index);
                }
            }
        } else {
            // No need to rehash any keys.
            keys.sweep_values(compactions);
            for vec in values.iter_mut() {
                vec.sweep_values(compactions);
            }
        }
    }
}
